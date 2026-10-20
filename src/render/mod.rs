//! Rendering backend abstraction.

mod cpu;
pub mod vulkan;

use anyhow::Result;

pub struct WindowSurface<'a> {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],
}

pub trait Renderer {
    fn modeset(&mut self) -> Result<()>;
    fn render(
        &mut self,
        frame: u64,
        windows: &[WindowSurface],
        cursor: Option<(i32, i32)>,
    ) -> Result<()>;
    fn flip(&mut self) -> Result<()>;
}
