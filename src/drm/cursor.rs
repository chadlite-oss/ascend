use anyhow::{Context, Result};
use drm::buffer::Buffer as DrmBuffer;
use drm::control::{Device as ControlDevice, crtc, dumbbuffer::DumbBuffer};
use drm_fourcc::DrmFourcc;

use super::Card;

pub struct DrmCursor {
    dumb: DumbBuffer,
    pub crtc: crtc::Handle,
}

impl DrmCursor {
    pub fn new(card: &Card, crtc: crtc::Handle) -> Result<Self> {
        // Standard Linux DRM cursor size is 64x64
        const SIZE: u32 = 64;
        let mut dumb = card
            .create_dumb_buffer((SIZE, SIZE), DrmFourcc::Argb8888, 32)
            .context("Failed to create dumb buffer for cursor")?;

        let pitch = DrmBuffer::pitch(&dumb) as usize;

        // Fill with classic arrow cursor bitmap
        {
            let mut mapping = card
                .map_dumb_buffer(&mut dumb)
                .context("Failed to map cursor dumb buffer")?;

            // Initialize all 64x64 pixels to fully transparent (Alpha = 0)
            mapping.fill(0);

            // Classic 16x24 arrow bitmap
            // '0' = Transparent
            // '1' = Black outline
            // '2' = Crisp white pointer body
            const ARROW: [&str; 24] = [
                "1000000000000000",
                "1100000000000000",
                "1210000000000000",
                "1221000000000000",
                "1222100000000000",
                "1222210000000000",
                "1222221000000000",
                "1222222100000000",
                "1222222210000000",
                "1222222221000000",
                "1222222222100000",
                "1222222222210000",
                "1222221111110000",
                "1222121000000000",
                "1221012100000000",
                "1210012100000000",
                "1100001210000000",
                "1000001210000000",
                "0000000121000000",
                "0000000121000000",
                "0000000011000000",
                "0000000000000000",
                "0000000000000000",
                "0000000000000000",
            ];

            for (y, row) in ARROW.iter().enumerate() {
                for (x, ch) in row.chars().enumerate() {
                    let idx = y * pitch + x * 4;
                    match ch {
                        '1' => {
                            // Solid black border: B=0, G=0, R=0, A=255
                            mapping[idx] = 0;
                            mapping[idx + 1] = 0;
                            mapping[idx + 2] = 0;
                            mapping[idx + 3] = 255;
                        }
                        '2' => {
                            // Solid white pointer body: B=255, G=255, R=255, A=255
                            mapping[idx] = 255;
                            mapping[idx + 1] = 255;
                            mapping[idx + 2] = 255;
                            mapping[idx + 3] = 255;
                        }
                        _ => {
                            // Transparent: A=0
                            mapping[idx] = 0;
                            mapping[idx + 1] = 0;
                            mapping[idx + 2] = 0;
                            mapping[idx + 3] = 0;
                        }
                    }
                }
            }
        }

        // Attach cursor to CRTC in hardware (with hotspot at (0, 0))
        #[allow(deprecated)]
        card.set_cursor2(crtc, Some(&dumb), (0, 0))
            .context("Failed to set DRM hardware cursor")?;

        Ok(Self { dumb, crtc })
    }

    #[allow(deprecated)]
    pub fn show(&self, card: &Card) -> Result<()> {
        card.set_cursor2(self.crtc, Some(&self.dumb), (0, 0))
            .context("Failed to show DRM hardware cursor")?;
        Ok(())
    }

    #[allow(deprecated)]
    pub fn hide(&self, card: &Card) -> Result<()> {
        card.set_cursor(self.crtc, None::<&DumbBuffer>)
            .context("Failed to hide DRM hardware cursor")?;
        Ok(())
    }

    #[allow(deprecated)]
    pub fn move_to(&self, card: &Card, x: i32, y: i32) -> Result<()> {
        card.move_cursor(self.crtc, (x, y))
            .context("Failed to move DRM hardware cursor")?;
        Ok(())
    }
}
