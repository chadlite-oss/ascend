mod drm;
mod event_loop;
mod gbm;
mod render;
use anyhow::{Context, Result};

use drm::{Card, Output};
use render::CpuRenderer;

fn main() -> Result<()> {
    let card = Card::open("/dev/dri/card1")?;

    card.acquire_master_lock()?;

    let output = Output::find_connected(&card)?;
    output.describe();

    let mut renderer = CpuRenderer::new(card, output).context("Failed to create CPU renderer")?;

    event_loop::run(&mut renderer)?;
    Ok(())
}
