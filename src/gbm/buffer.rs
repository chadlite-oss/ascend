//! GBM buffer wrapper.

use std::os::fd::OwnedFd;

use anyhow::{Context, Result};
use drm_fourcc::DrmModifier;
use gbm::{BufferObject, BufferObjectFlags, Format as GbmFormat};

use super::GbmDevice;

/// A GPU buffer allocated via GBM.
pub struct GbmBuffer {
    pub bo: BufferObject<()>,
    pub width: u32,
    pub height: u32,
    pub format: GbmFormat,
    pub modifier: DrmModifier,
    pub stride: u32,
    pub plane_count: u32,
}

impl GbmBuffer {
    /// Allocate a new GBM buffer suitable for rendering and scanout.
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

        let modifier = bo.modifier();
        let stride = bo.stride();
        let plane_count = bo.plane_count();

        Ok(Self {
            bo,
            width,
            height,
            format: GbmFormat::Xrgb8888,
            modifier,
            stride,
            plane_count,
        })
    }

    /// Export the buffer's memory as a DMA-BUF file descriptor.
    ///
    /// Each call returns a fresh fd. The returned `OwnedFd` closes it on drop.
    pub fn dma_buf_fd(&self) -> Result<OwnedFd> {
        self.bo.fd().context("Failed to get DMA-BUF fd")
    }
}
