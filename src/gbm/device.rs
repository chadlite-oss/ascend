//! GBM device wrapper.

use std::os::fd::{AsFd, OwnedFd};

use anyhow::{Context, Result};
use gbm::Device as GbmDeviceRaw;

use crate::drm::Card;

/// A wrapper around a GBM device, created from a DRM card.
pub struct GbmDevice {
    inner: GbmDeviceRaw<OwnedFd>,
}

impl GbmDevice {
    pub fn new(card: &Card) -> Result<Self> {
        let owned_fd = card
            .as_fd()
            .try_clone_to_owned()
            .context("Failed to duplicate DRM fd for GBM")?;

        let inner = GbmDeviceRaw::new(owned_fd).context("Failed to create GBM device")?;

        Ok(Self { inner })
    }

    pub fn raw(&self) -> &GbmDeviceRaw<OwnedFd> {
        &self.inner
    }
}
