use std::os::fd::IntoRawFd;

use anyhow::{Context, Result, anyhow};
use ash::vk;

use super::VulkanContext;
use crate::gbm::GbmBuffer;

pub struct VulkanImage {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
    pub width: u32,
    pub height: u32,
}

impl VulkanImage {
    pub fn import_from_gbm(context: &VulkanContext, gbm_buffer: &GbmBuffer) -> Result<Self> {
        let width = gbm_buffer.width;
        let height = gbm_buffer.height;

        let plane_layout = vk::SubresourceLayout::default()
            .offset(0)
            .size(0)
            .row_pitch(gbm_buffer.stride as u64);

        let modifier_val = match gbm_buffer.modifier {
            drm_fourcc::DrmModifier::Invalid => 0, // DRM_FORMAT_MOD_LINEAR
            m => u64::from(m),
        };

        let mut drm_modifier_info = vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
            .drm_format_modifier(modifier_val)
            .plane_layouts(std::slice::from_ref(&plane_layout));

        let mut external_memory_info = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

        let image_create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::B8G8R8A8_UNORM)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .push_next(&mut external_memory_info)
            .push_next(&mut drm_modifier_info);

        let image = unsafe { context.device.create_image(&image_create_info, None) }
            .context("Failed to create external Vulkan image")?;

        let mem_reqs = unsafe { context.device.get_image_memory_requirements(image) };

        let ext_mem_loader =
            ash::khr::external_memory_fd::Device::new(&context.instance, &context.device);

        // Clone the file descriptor using std::os::fd::OwnedFd::try_clone
        let dup_fd = gbm_buffer
            .fd
            .try_clone()
            .context("Failed to clone DMA-BUF fd")?;
        let raw_dup_fd = dup_fd.into_raw_fd();

        let mut fd_properties = vk::MemoryFdPropertiesKHR::default();
        unsafe {
            ext_mem_loader.get_memory_fd_properties(
                vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT,
                raw_dup_fd,
                &mut fd_properties,
            )
        }
        .context("Failed to query memory properties for DMA-BUF fd")?;

        let mem_props = unsafe {
            context
                .instance
                .get_physical_device_memory_properties(context.physical_device)
        };

        let memory_type_bits = mem_reqs.memory_type_bits & fd_properties.memory_type_bits;
        let memory_type_index = (0..mem_props.memory_type_count)
            .find(|&i| (memory_type_bits & (1 << i)) != 0)
            .ok_or_else(|| anyhow!("No compatible memory type found for imported DMA-BUF"))?;

        let mut import_info = vk::ImportMemoryFdInfoKHR::default()
            .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
            .fd(raw_dup_fd);

        let mut dedicated_alloc_info = vk::MemoryDedicatedAllocateInfo::default().image(image);

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(memory_type_index)
            .push_next(&mut import_info)
            .push_next(&mut dedicated_alloc_info);

        let memory = unsafe { context.device.allocate_memory(&alloc_info, None) }
            .context("Failed to allocate imported memory from DMA-BUF")?;

        unsafe { context.device.bind_image_memory(image, memory, 0) }
            .context("Failed to bind imported memory to Vulkan image")?;

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::B8G8R8A8_UNORM)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = unsafe { context.device.create_image_view(&view_info, None) }
            .context("Failed to create Vulkan image view")?;

        Ok(Self {
            image,
            memory,
            image_view,
            width,
            height,
        })
    }

    pub fn destroy(&mut self, context: &VulkanContext) {
        unsafe {
            context.device.destroy_image_view(self.image_view, None);
            context.device.destroy_image(self.image, None);
            context.device.free_memory(self.memory, None);
        }
    }
}
