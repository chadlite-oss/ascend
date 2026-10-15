use std::fs::OpenOptions;
use std::os::fd::{AsFd, BorrowedFd};

use anyhow::{Context, Result};
use drm::Device;
use drm::control::{Device as ControlDevice, ResourceHandles};

pub struct Card(std::fs::File);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl Device for Card {}
impl ControlDevice for Card {}

impl Card {
    pub fn open(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("Failed to open {path}"))?;
        Ok(Self(file))
    }

    pub fn acquire_master_lock(&self) -> Result<()> {
        Device::acquire_master_lock(self)
            .context("Failed to acquire DRM master lock (are you on a bare TTY?)")
    }
    pub fn release_master_lock(&self) -> Result<()> {
        Device::release_master_lock(self).context("Failed to release DRM master lock")
    }

    pub fn resources(&self) -> Result<ResourceHandles> {
        ControlDevice::resource_handles(self).context("Failed to get DRM resources")
    }
}
