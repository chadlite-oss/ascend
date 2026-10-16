use anyhow::{Context, Result};
use drm::control::{Device as ControlDevice, PageFlipFlags};

use crate::drm::{Card, DisplayBuffer, Output, wait_for_event};

use super::Renderer;

pub struct CpuRenderer {
    card: Card,
    buffers: [DisplayBuffer; 2],
    back: usize,
    output: Output,
}

impl CpuRenderer {
    pub fn new(card: Card, output: Output) -> Result<Self> {
        let buf0 = DisplayBuffer::new(&card, output.width, output.height)?;
        let buf1 = DisplayBuffer::new(&card, output.width, output.height)?;
        Ok(Self {
            card,
            buffers: [buf0, buf1],
            back: 0,
            output,
        })
    }

    fn draw_gradient(&mut self, idx: usize, frame: u64) -> Result<()> {
        let width = self.buffers[idx].width as usize;
        let height = self.buffers[idx].height as usize;
        let phase = ((frame * 4) % width as u64) as usize;

        let CpuRenderer { card, buffers, .. } = self;
        let buffer = &mut buffers[idx];

        let mut mapping = buffer.map(card)?;
        let buf = mapping.as_mut();

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let sx = (x + phase) % width;
                let sy = (y + phase) % height;

                buf[idx + 0] = (sx * 255 / width) as u8;
                buf[idx + 1] = (sy * 255 / height) as u8;
                buf[idx + 2] = 128;
                buf[idx + 3] = 0;
            }
        }
        Ok(())
    }
}

impl Renderer for CpuRenderer {
    fn modeset(&mut self) -> Result<()> {
        self.draw_gradient(0, 0)?;

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
        self.draw_gradient(back, frame)
    }

    fn flip(&mut self) -> Result<()> {
        let fb = self.buffers[self.back].fb;

        self.card
            .page_flip(self.output.crtc, fb, PageFlipFlags::EVENT, None)
            .context("page_flip falied")?;

        wait_for_event(&self.card)?;

        for event in self.card.receive_events()? {
            if let drm::control::Event::PageFlip(_) = event {}
        }

        self.back = 1 - self.back;
        Ok(())
    }
}
