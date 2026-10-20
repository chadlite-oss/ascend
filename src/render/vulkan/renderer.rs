use anyhow::{Context, Result};
use ash::vk;
use drm::control::{Device as ControlDevice, PageFlipFlags, framebuffer};

use super::{VulkanContext, VulkanImage};
use crate::drm::{Card, Output, wait_for_event};
use crate::gbm::{GbmBuffer, GbmDevice};
use crate::render::{Renderer, WindowSurface};

struct FrameBuffer {
    _gbm_buffer: GbmBuffer,
    fb: framebuffer::Handle,
    vulkan_image: VulkanImage,
}

pub struct VulkanRenderer {
    card: Card,
    _gbm_device: GbmDevice,
    context: VulkanContext,
    buffers: [FrameBuffer; 2],
    back: usize,
    output: Output,
    cmd_buffers: Vec<vk::CommandBuffer>,
    fence: vk::Fence,
    staging_buffer: vk::Buffer,
    staging_memory: vk::DeviceMemory,
    staging_capacity: usize,
}

impl VulkanRenderer {
    pub fn new(card: Card, output: Output) -> Result<Self> {
        let gbm_device = GbmDevice::new(&card)?;
        let context = VulkanContext::new()?;

        let width = output.width;
        let height = output.height;

        let gbm0 = GbmBuffer::new(&gbm_device, width, height)?;
        let fb0 = gbm0.add_framebuffer(&card)?;
        let img0 = VulkanImage::import_from_gbm(&context, &gbm0)?;

        let gbm1 = GbmBuffer::new(&gbm_device, width, height)?;
        let fb1 = gbm1.add_framebuffer(&card)?;
        let img1 = VulkanImage::import_from_gbm(&context, &gbm1)?;

        let buffers = [
            FrameBuffer {
                _gbm_buffer: gbm0,
                fb: fb0,
                vulkan_image: img0,
            },
            FrameBuffer {
                _gbm_buffer: gbm1,
                fb: fb1,
                vulkan_image: img1,
            },
        ];

        let cmd_buffers = context.allocate_command_buffer(2)?;

        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
        let fence = unsafe { context.device.create_fence(&fence_info, None) }
            .context("Failed to create Vulkan fence")?;

        // Allocate a staging buffer large enough to hold a full screen of RGBA pixels
        let staging_capacity = (width * height * 4) as usize;
        let (staging_buffer, staging_memory) = context.create_buffer(
            staging_capacity as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        Ok(Self {
            card,
            _gbm_device: gbm_device,
            context,
            buffers,
            back: 0,
            output,
            cmd_buffers,
            fence,
            staging_buffer,
            staging_memory,
            staging_capacity,
        })
    }

    fn record_and_submit(
        &mut self,
        idx: usize,
        frame: u64,
        windows: &[WindowSurface],
        cursor: Option<(i32, i32)>,
    ) -> Result<()> {
        let cmd = self.cmd_buffers[idx];
        let image = self.buffers[idx].vulkan_image.image;
        let device = &self.context.device;

        unsafe {
            device.wait_for_fences(&[self.fence], true, u64::MAX)?;
            device.reset_fences(&[self.fence])?;
        }

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device.begin_command_buffer(cmd, &begin_info)?;

            let subresource_range = vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            };

            let to_transfer_barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .image(image)
                .subresource_range(subresource_range);

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[to_transfer_barrier],
            );

            // 1. Clear background with animated color gradient
            let t = frame as f32 * 0.03;
            let r = (t.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            let g = ((t + 2.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            let b = ((t + 4.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);

            let clear_color = vk::ClearColorValue {
                float32: [r, g, b, 1.0],
            };

            device.cmd_clear_color_image(
                cmd,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &clear_color,
                &[subresource_range],
            );

            // 2. Paste client windows onto the screen
            let mut staging_offset: usize = 0;
            for window in windows {
                let win_w = window.width.min(self.output.width);
                let win_h = window.height.min(self.output.height);
                let needed_bytes = (win_w * win_h * 4) as usize;

                if needed_bytes > 0
                    && staging_offset + needed_bytes <= self.staging_capacity
                    && window.pixels.len() >= needed_bytes
                {
                    self.context.write_to_memory_offset(
                        self.staging_memory,
                        staging_offset as u64,
                        &window.pixels[..needed_bytes],
                    )?;

                    let clear_to_copy_barrier = vk::ImageMemoryBarrier::default()
                        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .image(image)
                        .subresource_range(subresource_range);

                    device.cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[clear_to_copy_barrier],
                    );

                    let copy_region = vk::BufferImageCopy::default()
                        .buffer_offset(staging_offset as u64)
                        .buffer_row_length(win_w)
                        .buffer_image_height(win_h)
                        .image_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .image_offset(vk::Offset3D {
                            x: window.x.max(0),
                            y: window.y.max(0),
                            z: 0,
                        })
                        .image_extent(vk::Extent3D {
                            width: win_w,
                            height: win_h,
                            depth: 1,
                        });

                    device.cmd_copy_buffer_to_image(
                        cmd,
                        self.staging_buffer,
                        image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[copy_region],
                    );

                    staging_offset += needed_bytes;
                }
            }

            // 3. Draw cursor on top
            if let Some((cx, cy)) = cursor {
                const CURSOR_SIZE: u32 = 16;
                let mut cursor_pixels = [255u8; (CURSOR_SIZE * CURSOR_SIZE * 4) as usize];
                for y in 0..CURSOR_SIZE {
                    for x in 0..CURSOR_SIZE {
                        let idx = ((y * CURSOR_SIZE + x) * 4) as usize;
                        let is_border =
                            x == 0 || y == 0 || x == CURSOR_SIZE - 1 || y == CURSOR_SIZE - 1;
                        let is_cross = (x == CURSOR_SIZE / 2) || (y == CURSOR_SIZE / 2);
                        if is_border || is_cross {
                            cursor_pixels[idx] = 255;
                            cursor_pixels[idx + 1] = 255;
                            cursor_pixels[idx + 2] = 255;
                            cursor_pixels[idx + 3] = 255;
                        } else {
                            cursor_pixels[idx] = 255;
                            cursor_pixels[idx + 1] = 40;
                            cursor_pixels[idx + 2] = 40;
                            cursor_pixels[idx + 3] = 255;
                        }
                    }
                }

                let cursor_bytes = (CURSOR_SIZE * CURSOR_SIZE * 4) as usize;
                if staging_offset + cursor_bytes <= self.staging_capacity {
                    self.context.write_to_memory_offset(
                        self.staging_memory,
                        staging_offset as u64,
                        &cursor_pixels,
                    )?;

                    let cursor_barrier = vk::ImageMemoryBarrier::default()
                        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .image(image)
                        .subresource_range(subresource_range);

                    device.cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[cursor_barrier],
                    );

                    let copy_region = vk::BufferImageCopy::default()
                        .buffer_offset(staging_offset as u64)
                        .buffer_row_length(CURSOR_SIZE)
                        .buffer_image_height(CURSOR_SIZE)
                        .image_subresource(vk::ImageSubresourceLayers {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            mip_level: 0,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .image_offset(vk::Offset3D {
                            x: cx.clamp(0, (self.output.width.saturating_sub(CURSOR_SIZE)) as i32),
                            y: cy.clamp(0, (self.output.height.saturating_sub(CURSOR_SIZE)) as i32),
                            z: 0,
                        })
                        .image_extent(vk::Extent3D {
                            width: CURSOR_SIZE,
                            height: CURSOR_SIZE,
                            depth: 1,
                        });

                    device.cmd_copy_buffer_to_image(
                        cmd,
                        self.staging_buffer,
                        image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[copy_region],
                    );
                }
            }

            // 4. Transition to general layout for monitor scanout
            let to_scanout_barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .image(image)
                .subresource_range(subresource_range);

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[to_scanout_barrier],
            );

            device.end_command_buffer(cmd)?;

            let submit_info = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));

            device.queue_submit(self.context.queue, &[submit_info], self.fence)?;
        }
        Ok(())
    }
}

impl Renderer for VulkanRenderer {
    fn modeset(&mut self) -> Result<()> {
        self.record_and_submit(0, 0, &[], None)?;

        unsafe {
            self.context
                .device
                .wait_for_fences(&[self.fence], true, u64::MAX)?;
        }

        self.card
            .set_crtc(
                self.output.crtc,
                Some(self.buffers[0].fb),
                (0, 0),
                &[self.output.connector],
                Some(self.output.mode),
            )
            .context("set_crtc failed")?;

        self.back = 1;
        Ok(())
    }

    fn render(
        &mut self,
        frame: u64,
        windows: &[WindowSurface],
        cursor: Option<(i32, i32)>,
    ) -> Result<()> {
        let back = self.back;
        self.record_and_submit(back, frame, windows, cursor)
    }

    fn flip(&mut self) -> Result<()> {
        unsafe {
            self.context
                .device
                .wait_for_fences(&[self.fence], true, u64::MAX)?;
        }

        let fb = self.buffers[self.back].fb;

        self.card
            .page_flip(self.output.crtc, fb, PageFlipFlags::EVENT, None)
            .context("page_flip failed")?;

        wait_for_event(&self.card)?;

        for event in self.card.receive_events()? {
            if let drm::control::Event::PageFlip(_) = event {}
        }

        self.back = 1 - self.back;
        Ok(())
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.context.device.device_wait_idle();
            self.context
                .device
                .destroy_buffer(self.staging_buffer, None);
            self.context.device.free_memory(self.staging_memory, None);
            self.context.device.destroy_fence(self.fence, None);
            for buf in &mut self.buffers {
                buf.vulkan_image.destroy(&self.context);
            }
        }
    }
}
