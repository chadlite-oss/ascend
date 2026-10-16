//! GBM (Generic Buffer Manager) layer.
//!
//! GBM allocates GPU buffers that can be used for rendering (Vulkan) and
//! scanout (DRM/KMS). The DMA-BUF fd is the zero-copy bridge between them.

mod buffer;
mod device;

pub use buffer::GbmBuffer;
pub use device::GbmDevice;
