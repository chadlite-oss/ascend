//! The central event loop.

use anyhow::Result;

use crate::render::Renderer;
use crate::wayland::WaylandServer;
pub fn run(renderer: &mut impl Renderer, wayland: &mut WaylandServer) -> Result<()> {
    println!("Setting mode...");
    renderer.modeset()?;

    println!(
        "Ascend compositor running! Wayland socket: {:?}",
        wayland.socket_name
    );
    println!("Press Ctrl+C to stop.");
    println!();

    let mut frame: u64 = 0;
    loop {
        wayland.dispatch()?;
        renderer.render(frame)?;
        renderer.flip()?;

        if frame % 288 == 0 {
            println!("frame {}", frame);
        }

        frame += 1;
    }
}
