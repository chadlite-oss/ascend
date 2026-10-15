use anyhow::{Context, Result};
use drm::buffer::Buffer as DrmBuffer;
use drm::control::{
    Device as ControlDevice,
    dumbbuffer::{DumbBuffer, DumbMapping},
    framebuffer,
};
use drm_fourcc::DrmFourcc;

use super::Card;

pub struct DisplayBuffer {
    pub dumb: DumbBuffer,
    pub fb: framebuffer::Handle,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub size: usize,
}

impl DisplayBuffer {
    pub fn new(card: &Card, width: u32, height: u32) -> Result<Self> {
        let dumb = card
            .create_dumb_buffer((width, height), DrmFourcc::Xrgb8888, 32)
            .context("Failed to create dumb buffer")?;

        let stride = DrmBuffer::pitch(&dumb);
        let size = stride as usize * height as usize;

        let fb = card
            .add_framebuffer(&dumb, 24, 32)
            .context("Failed to add framebuffer")?;

        Ok(Self {
            dumb,
            fb,
            width,
            height,
            stride,
            size,
        })
    }

    pub fn map(&mut self, card: &Card) -> Result<DumbMapping<'_>> {
        card.map_dumb_buffer(&mut self.dumb)
            .context("Failed to map dumb buffer")
    }
}
