//! GBM buffer wrapper.

use std::os::fd::{AsRawFd, OwnedFd, RawFd};

use anyhow::{Context, Result};
use drm::buffer::{Buffer as DrmBuffer, PlanarBuffer};
use drm::control::{Device as ControlDevice, FbCmd2Flags, framebuffer};
use drm_fourcc::{DrmFourcc, DrmModifier};
use gbm::{BufferObject, BufferObjectFlags, Format as GbmFormat};
use std::num::NonZeroU32;

use super::GbmDevice;
use crate::drm::Card;

fn fd_to_handle(fd: RawFd) -> drm::buffer::Handle {
    NonZeroU32::new(fd as u32)
        .expect("file descriptor is always positive")
        .into()
}

pub struct GbmBuffer {
    pub bo: BufferObject<()>,
    pub fd: OwnedFd,
    pub width: u32,
    pub height: u32,
    pub format: GbmFormat,
    pub modifier: DrmModifier,
    pub stride: u32,
    pub plane_count: u32,
}

// ============================================================================
// Inherent methods
// ============================================================================
impl GbmBuffer {
    pub fn new(device: &GbmDevice, width: u32, height: u32) -> Result<Self> {
        let bo = device
            .raw()
            .create_buffer_object::<()>(
                width,
                height,
                GbmFormat::Xrgb8888,
                BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING,
            )
            .context("Failed to create GBM buffer object")?;

        let fd = bo.fd().context("Failed to get DMA-BUF fd")?;
        let modifier = bo.modifier();
        let stride = bo.stride();
        let plane_count = bo.plane_count();

        Ok(Self {
            bo,
            fd,
            width,
            height,
            format: GbmFormat::Xrgb8888,
            modifier,
            stride,
            plane_count,
        })
    }

    pub fn dma_buf_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn add_framebuffer(&self, card: &Card) -> Result<framebuffer::Handle> {
        card.add_planar_framebuffer(self, FbCmd2Flags::MODIFIERS)
            .context("Failed to register GBM buffer as DRM framebuffer")
    }
}

// ============================================================================
// drm::buffer::Buffer — simple single-plane description
// ============================================================================
impl DrmBuffer for GbmBuffer {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn format(&self) -> DrmFourcc {
        DrmFourcc::Xrgb8888
    }

    fn pitch(&self) -> u32 {
        self.stride
    }

    fn handle(&self) -> drm::buffer::Handle {
        fd_to_handle(self.fd.as_raw_fd())
    }
}
// ============================================================================
// drm::buffer::PlanarBuffer — multi-plane description
// ============================================================================
impl PlanarBuffer for GbmBuffer {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn format(&self) -> DrmFourcc {
        DrmFourcc::Xrgb8888
    }

    fn modifier(&self) -> Option<DrmModifier> {
        Some(self.modifier)
    }

    fn pitches(&self) -> [u32; 4] {
        [self.stride, 0, 0, 0]
    }

    fn handles(&self) -> [Option<drm::buffer::Handle>; 4] {
        [Some(fd_to_handle(self.fd.as_raw_fd())), None, None, None]
    }

    fn offsets(&self) -> [u32; 4] {
        [0, 0, 0, 0]
    }
}
