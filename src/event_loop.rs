//! The central event loop.

use anyhow::Result;
use wayland_server::Resource;

use crate::render::{Renderer, WindowSurface};
use crate::wayland::{ShmBufferData, WaylandServer};

pub fn run(
    renderer: &mut impl Renderer,
    wayland: &mut WaylandServer,
    input: &mut crate::input::InputManager,
) -> Result<()> {
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
        // 1. Process client requests and connections
        wayland.dispatch()?;

        // 2. Process hardware mouse/keyboard events
        input.dispatch()?;

        // 3. Collect active client windows
        let mut windows = Vec::new();
        for surf in &wayland.state.surface {
            if let Some(buf) = &surf.buffer {
                if let Some(buf_data) = buf.data::<ShmBufferData>() {
                    windows.push(WindowSurface {
                        x: surf.x,
                        y: surf.y,
                        width: buf_data.width as u32,
                        height: buf_data.height as u32,
                        pixels: buf_data.pixel_data(),
                    });
                }
            }
        }

        // 4. Render windows + cursor
        let cursor = Some((input.cursor_x as i32, input.cursor_y as i32));
        renderer.render(frame, &windows, cursor)?;
        renderer.flip()?;

        if frame % 288 == 0 {
            println!("frame {}", frame);
        }

        frame += 1;
    }
}
