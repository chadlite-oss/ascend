mod drm;
mod event_loop;
mod gbm;
mod render;

use anyhow::Result;

fn main() -> Result<()> {
    // Temporary: prove Vulkan works before wiring it into the renderer.
    let _vulkan = render::VulkanContext::new()?;
    println!("Vulkan context created successfully");

    /*
    let card = drm::Card::open("/dev/dri/card1")?;
    card.acquire_master_lock()?;
    let output = drm::Output::find_connected(&card)?;
    output.describe();
    let mut renderer = render::CpuRenderer::new(card, output)?;
    event_loop::run(&mut renderer)?;
    */

    Ok(())
}
