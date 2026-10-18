use anyhow::{Context, Result};
use ash::vk;
use drm::control::{Device as ControlDevice, PageFlipFlags, framebuffer};

use super::{VulkanContext, VulkanImage};
use crate::drm::{Card, Output, wait_for_event};
use crate::gbm::{GbmBuffer, GbmDevice};
use crate::render::Renderer;

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

        Ok(Self {
            card,
            _gbm_device: gbm_device,
            context,
            buffers,
            back: 0,
            output,
            cmd_buffers,
            fence,
        })
    }

    fn record_and_submit_clear(&mut self, idx: usize, frame: u64) -> Result<()> {
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

            let submit_info =
                vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));

            device.queue_submit(self.context.queue, &[submit_info], self.fence)?;
        }
        Ok(())
    }
}

impl Renderer for VulkanRenderer {
    fn modeset(&mut self) -> Result<()> {
        self.record_and_submit_clear(0, 0)?;

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

    fn render(&mut self, frame: u64) -> Result<()> {
        let back = self.back;
        self.record_and_submit_clear(back, frame)
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
            self.context.device.destroy_fence(self.fence, None);
            for buf in &mut self.buffers {
                buf.vulkan_image.destroy(&self.context);
            }
        }
    }
}
