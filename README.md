## AI Declaration

AI tools were used to assist with parts of the development process .

# Ascend

 A wayland compositor that made in rust very simple

## Core Features

### 1. Direct DRM/KMS Modesetting


### 2. Zero-Copy Vulkan Scanout Engine


### 3. Native Input Management 


### 4. Wayland Protocol Server 


### 5. Robust Logging Subsystem 


![picture just color full image for now](examples/photo_2026-10-21_00-20-39.jpg)
![see the video](examples/demo3.gif)
---

## Roadmap

- [x] **v0.1:** DRM/KMS direct scanout modesetting and double-buffered page flipping.
- [x] **v0.2:** Zero-copy GBM DMA-BUF import into Vulkan and GPU-accelerated rendering.
- [x] **v0.3:** `libinput` multi-device input handling (pointer + keyboard) and DRM hardware cursor plane.
- [ ] **v0.4 (In Progress):** `xdg_shell` implementation to manage and tile interactive client windows (terminals, browsers, etc.).
- [ ] **v0.5:** Wayland keyboard focus, pointer focus, and seat capabilities (`wl_seat`).

---

## Getting Started

### System Requirements

- **Operating System:** Linux (Arch Linux, Fedora, Debian/Ubuntu, etc.)
- **GPU:** AMD (radv), Intel (anv), or NVIDIA with DRM/KMS and Vulkan driver support.
- **Dependencies:**
  - `libdrm`
  - `libgbm`
  - `libinput`
  - `libudev`
  - `vulkan-loader`

On Arch Linux:
```bash
sudo pacman -S vulkan-devel libinput libdrm mesa systemd
```

### Building

Debug build:
```bash
cargo build
```

Optimized release build:
```bash
cargo build --release
```

The resulting binary will be located at `target/release/ascend`.

---

## Running Ascend

Because Ascend directly accesses `/dev/dri/card*` and acquires the DRM master lock, it must be launched from a **bare TTY** (not from within an existing X11 or Wayland session):

1. Switch to a bare virtual console using **`Ctrl + Alt + F3`** (or `F4`, `F5`).
2. Log in and navigate to the project repository.
3. Start the compositor:

```bash
cargo run --release
```

4. To stop the compositor, press ctrl + c
