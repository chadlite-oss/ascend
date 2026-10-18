mod drm;
mod event_loop;
mod gbm;
mod render;

use anyhow::Result;

fn main() -> Result<()> {
    println!("1. Opening DRM card (/dev/dri/card1)...");
    let card = drm::Card::open("/dev/dri/card1")?;
    card.acquire_master_lock()?;

    println!("2. Finding connected display...");
    let output = drm::Output::find_connected(&card)?;
    output.describe();

    println!("3. Initializing Vulkan renderer...");
    let mut renderer = render::vulkan::VulkanRenderer::new(card, output)?;

    println!("4. Starting render loop...");
    event_loop::run(&mut renderer)?;

    Ok(())
}
