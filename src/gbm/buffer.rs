use std::os::fd::{AsRawFd, OwnedFd, RawFd};

use anyhow::{Context, Result};
use drm::control::{Device as ControlDevice, FbCmd2Flags, framebuffer};
use drm_fourcc::DrmModifier;
use gbm::{BufferObject, BufferObjectFlags, Format as GbmFormat};

use super::GbmDevice;
use crate::drm::Card;

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
        let flags = if self.modifier == DrmModifier::Invalid {
            FbCmd2Flags::empty()
        } else {
            FbCmd2Flags::MODIFIERS
        };
        card.add_planar_framebuffer(&self.bo, flags)
            .context("Failed to register GBM buffer as DRM framebuffer")
    }
}

// BufferObject already implements DrmBuffer and PlanarBuffer correctly
// using the driver's actual GEM handles via gbm_bo_get_handle_for_plane.
