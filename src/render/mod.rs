mod cpu;

pub use cpu::CpuRenderer;

use anyhow::Result;

pub trait Renderer {
    fn modeset(&mut self) -> Result<()>;

    fn render(&mut self, frame: u64) -> Result<()>;

    fn flip(&mut self) -> Result<()>;
}
