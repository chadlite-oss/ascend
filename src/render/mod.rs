//! Rendering backend abstraction.

mod cpu;
pub mod vulkan;

pub use cpu::CpuRenderer;
pub use vulkan::{VulkanContext, VulkanImage};

use anyhow::Result;

pub trait Renderer {
    fn modeset(&mut self) -> Result<()>;
    fn render(&mut self, frame: u64) -> Result<()>;
    fn flip(&mut self) -> Result<()>;
}
