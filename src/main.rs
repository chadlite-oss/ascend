#[macro_use]
mod logger;

mod drm;
mod event_loop;
mod gbm;
mod input;
mod render;
mod wayland;

use anyhow::Result;

fn main() -> Result<()> {
    logger::init()?;
    log_info!("=== Ascend Compositor Starting ===");

    if let Err(e) = run_app() {
        log_info!("FATAL: {:?}", e);
        return Err(e);
    }

    Ok(())
}

fn run_app() -> Result<()> {
    log_info!("1. Opening DRM card (/dev/dri/card1)...");
    let card = drm::Card::open("/dev/dri/card1")?;
    card.acquire_master_lock()?;

    log_info!("2. Finding connected display...");
    let output = drm::Output::find_connected(&card)?;
    output.describe();

    let width = output.width;
    let height = output.height;
    log_info!("Display resolution: {}x{}", width, height);

    log_info!("3. Initializing Vulkan renderer...");
    let mut renderer = render::vulkan::VulkanRenderer::new(card, output)?;

    log_info!("4. Initializing Wayland server...");
    let mut wayland = wayland::WaylandServer::new()?;

    log_info!("5. Initializing Input manager...");
    let mut input = input::InputManager::new(width, height)?;

    log_info!("6. Starting render loop...");
    event_loop::run(&mut renderer, &mut wayland, &mut input)?;

    Ok(())
}
