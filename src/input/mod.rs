use std::ffi::CString;
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use input::event::keyboard::{KeyboardEvent, KeyboardEventTrait};
use input::event::pointer::PointerEvent;
use input::event::Event;
use input::{Libinput, LibinputInterface};

struct DeviceOpener;

impl LibinputInterface for DeviceOpener {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> std::result::Result<OwnedFd, i32> {
        let c_path = CString::new(path.to_str().unwrap_or_default()).map_err(|_| libc::EINVAL)?;

        let fd = unsafe { libc::open(c_path.as_ptr(), flags) };
        if fd < 0 {
            let err = std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL);
            Err(err)
        } else {
            Ok(unsafe { OwnedFd::from_raw_fd(fd) })
        }
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(fd);
    }
}

pub struct InputManager {
    libinput: Libinput,
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub screen_width: u32,
    pub screen_height: u32,
}

impl InputManager {
    pub fn new(screen_width: u32, screen_height: u32) -> Result<Self> {
        let mut libinput = Libinput::new_with_udev(DeviceOpener);

        libinput
            .udev_assign_seat("seat0")
            .map_err(|_| anyhow!("Failed to assign udev seat 'seat0'"))?;

        Ok(Self {
            libinput,
            cursor_x: (screen_width / 2) as f64,
            cursor_y: (screen_height / 2) as f64,
            screen_width,
            screen_height,
        })
    }

    pub fn dispatch(&mut self) -> Result<()> {
        self.libinput
            .dispatch()
            .context("Failed to dispatch libinput")?;

        for event in &mut self.libinput {
            match event {
                Event::Pointer(PointerEvent::Motion(m)) => {
                    self.cursor_x += m.dx();
                    self.cursor_y += m.dy();
                }

                Event::Pointer(PointerEvent::MotionAbsolute(m)) => {
                    self.cursor_x = m.absolute_x_transformed(self.screen_width);
                    self.cursor_y = m.absolute_y_transformed(self.screen_height);
                }

                Event::Pointer(PointerEvent::Button(b)) => {
                    println!(
                        "Mouse button {:?} state: {:?}",
                        b.button(),
                        b.button_state()
                    );
                }

                Event::Keyboard(KeyboardEvent::Key(k)) => {
                    println!("Key: {:?} state: {:?}", k.key(), k.key_state());
                }

                _ => {}
            }
        }

        self.cursor_x = self
            .cursor_x
            .clamp(0.0, (self.screen_width.saturating_sub(1)) as f64);
        self.cursor_y = self
            .cursor_y
            .clamp(0.0, (self.screen_height.saturating_sub(1)) as f64);

        Ok(())
    }
}
