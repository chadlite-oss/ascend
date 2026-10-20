use std::ffi::CStr;

use anyhow::{Context, Result, anyhow};
use ash::vk;

pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub queue: vk::Queue,
    pub queue_family_index: u32,
    pub command_pool: vk::CommandPool,
}

impl VulkanContext {
    pub fn new() -> Result<Self> {
        let entry = unsafe { ash::Entry::load() }
            .context("Failed to load Vulkan library (libvulkan.so.1)")?;

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"ascend")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"ascend")
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::make_api_version(0, 1, 3, 0));

        let instance_extensions = [ash::khr::surface::NAME.as_ptr()];
        #[cfg(debug_assertions)]
        let layers = [c"VK_LAYER_KHRONOS_validation".as_ptr()];
        #[cfg(not(debug_assertions))]
        let layers: [*const std::os::raw::c_char; 0] = [];

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&instance_extensions)
            .enabled_layer_names(&layers);

        let instance = unsafe { entry.create_instance(&instance_create_info, None) }
            .context("Failed to create Vulkan instance")?;

        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .context("Failed to enumerate Vulkan physical devices")?;

        if physical_devices.is_empty() {
            return Err(anyhow!("No Vulkan-compatible GPU found"));
        }

        let required_device_extensions = [
            ash::khr::external_memory::NAME,
            ash::khr::external_memory_fd::NAME,
            ash::ext::external_memory_dma_buf::NAME,
            ash::ext::image_drm_format_modifier::NAME,
        ];

        let physical_device = physical_devices.into_iter().find(|&pd| {
            let props = unsafe { instance.enumerate_device_extension_properties(pd) }.unwrap_or_default();
            required_device_extensions.iter().all(|&req| {
                props.iter().any(|p| {
                    let name = unsafe {
                        CStr::from_ptr(p.extension_name.as_ptr())
                    };
                    name == req
                })
            })
        })
        .ok_or_else(|| anyhow!(
            "No GPU supported VK_EXT_external_memory_dma_buf + VK_EXT_image_drm_format_modifier"
        ))?;

        let queue_props =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let queue_family_index = queue_props
            .iter()
            .position(|props| props.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .ok_or_else(|| anyhow!("No graphic queue family found"))?
            as u32;

        let queue_priorities = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);

        let device_extension_ptrs: Vec<*const std::os::raw::c_char> = required_device_extensions
            .iter()
            .map(|&name| name.as_ptr())
            .collect();

        let mut dynamic_rendering_feature =
            vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_create_info))
            .enabled_extension_names(&device_extension_ptrs)
            .push_next(&mut dynamic_rendering_feature);

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }
            .context("Failed to create Vulkan logical device")?;
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.create_command_pool(&pool_info, None) }
            .context("Failed to create command pool")?;
        println!("Vulkan initialized with command pool and DMA-BUF extensions.");
        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            queue,
            queue_family_index,
            command_pool,
        })
    }

    pub fn allocate_command_buffer(&self, count: u32) -> Result<Vec<vk::CommandBuffer>> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);

        unsafe {
            self.device
                .allocate_command_buffers(&alloc_info)
                .context("Failed to allocate command buffers")
        }
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe { self.device.device_wait_idle() }.context("Failed to wait for device idle")
    }

    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        let mem_props = unsafe {
            self.instance
                .get_physical_device_memory_properties(self.physical_device)
        };

        for i in 0..mem_props.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && mem_props.memory_types[i as usize]
                    .property_flags
                    .contains(properties)
            {
                return Some(i);
            }
        }
        None
    }

    pub fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe { self.device.create_buffer(&buffer_info, None) }
            .context("Failed to create Vulkan buffer")?;

        let mem_reqs = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let memory_type_index = self
            .find_memory_type(mem_reqs.memory_type_bits, properties)
            .ok_or_else(|| anyhow!("Failed to find suitable memory type for buffer"))?;

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe { self.device.allocate_memory(&alloc_info, None) }
            .context("Failed to allocate buffer memory")?;

        unsafe { self.device.bind_buffer_memory(buffer, memory, 0) }
            .context("Failed to bind buffer memory")?;

        Ok((buffer, memory))
    }

    pub fn write_to_memory(&self, memory: vk::DeviceMemory, data: &[u8]) -> Result<()> {
        self.write_to_memory_offset(memory, 0, data)
    }

    pub fn write_to_memory_offset(
        &self,
        memory: vk::DeviceMemory,
        offset: u64,
        data: &[u8],
    ) -> Result<()> {
        unsafe {
            let ptr = self.device.map_memory(
                memory,
                offset,
                data.len() as u64,
                vk::MemoryMapFlags::empty(),
            )?;

            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
            self.device.unmap_memory(memory);
        }
        Ok(())
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
