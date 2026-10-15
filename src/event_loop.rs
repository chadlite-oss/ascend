//! The central event loop.

use anyhow::Result;

use crate::render::Renderer;

pub fn run(renderer: &mut impl Renderer) -> Result<()> {
    println!("Setting mode...");
    renderer.modeset()?;

    println!("Animating. Press Ctrl+C to stop.");
    println!();

    let mut frame: u64 = 0;
    loop {
        renderer.render(frame)?;
        renderer.flip()?;

        // Print a heartbeat every ~2 seconds at 144Hz.
        if frame % 288 == 0 {
            println!("frame {}", frame);
        }

        frame += 1;
    }
}
