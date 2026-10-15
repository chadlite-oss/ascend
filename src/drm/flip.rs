use std::os::fd::AsFd;

use anyhow::{Context, Result};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

use super::Card;

pub fn wait_for_event(card: &Card) -> Result<()> {
    let fd = card.as_fd();
    let mut poll_fds = [PollFd::new(fd, PollFlags::POLLIN)];
    poll(&mut poll_fds, PollTimeout::NONE).context("poll() on DRM fd failed")?;
    Ok(())
}
