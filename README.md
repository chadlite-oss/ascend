# Ascend

A modern, high-performance Wayland compositor written from scratch in Rust, featuring direct Linux DRM/KMS modesetting, a zero-copy Vulkan graphics pipeline via GBM DMA-BUF, dedicated hardware cursor planes, and native `libinput` event dispatching.

---

## Architecture Overview

Ascend is designed without relying on large, monolithic compositor abstraction libraries (such as `wlroots` or `smithay`). Instead, it directly interfaces with Linux kernel subsystems and low-level graphics APIs to provide minimal latency, maximum predictability, and complete architectural transparency:

```
+-------------------------------------------------------------+
|                     Ascend Event Loop                       |
+-------------+---------------------+-------------------------+
|             |                     |                         |
|   Wayland   |      libinput       |     Vulkan Renderer     |
|   Server    |      Manager        |     & GBM Pipeline      |
| (wl_shm,    | (relative/absolute  | (DMA-BUF zero-copy      |
|  surfaces)  |  motion, buttons)   |  compositor scanout)    |
|             |                     |                         |
+------+------+----------+----------+------------+------------+
       |                 |                       |
       |                 |            +----------v----------+
       |                 |            |  DRM Hardware Plane |
       |                 |            |  & CRTC Modesetting |
       v                 v            | (Hardware Cursor +  |
+-----------------------------------+ |  Primary Scanout)   |
|   Linux Kernel Subsystems         | +----------+----------+
|   (KMS, DRM, evdev, udev, uapi)   |            |
+-----------------------------------+------------v----------+
                                    |     Display Output    |
                                    |     (e.g. 2560x1440)  |
                                    +-----------------------+
```

---

## Core Features

### 1. Direct DRM/KMS Modesetting ([`src/drm`](src/drm))
- Directly handles Linux Direct Rendering Manager (`/dev/dri/card*`) device discovery and master lock acquisition.
- Manages connector enumeration, CRTC mode selection, and double-buffered page flips synchronized with hardware vblank events (`PageFlipFlags::EVENT`).
- **DRM Hardware Cursor Plane ([`src/drm/cursor.rs`](src/drm/cursor.rs)):** Allocates dedicated $64 \times 64$ ARGB8888 dumb buffers for the mouse pointer. The pointer is scanned out directly by GPU display hardware with true alpha blending, eliminating software blitting overhead and bounding box artifacts.

### 2. Zero-Copy Vulkan Scanout Engine ([`src/render/vulkan`](src/render/vulkan))
- **Generic Buffer Management (GBM):** Allocates GPU scanout framebuffers with driver-native DRM format modifiers (with linear fallback).
- **DMA-BUF Import:** Imports GBM buffers into Vulkan as `vk::Image` instances using `VK_EXT_external_memory_dma_buf` and `VK_EXT_image_drm_format_modifier`.
- **Hardware Command Submission:** Coordinates Vulkan command buffers, memory barriers, and synchronization fences (`vk::Fence`) to composite client windows on top of dynamic desktop backgrounds at monitor refresh rates.

### 3. Native Input Management ([`src/input`](src/input))
- Interacts directly with kernel `evdev` devices via `libinput` under seat `seat0`.
- Tracks both relative pointer motion (mice) and absolute pointer transformations (touchpads/tablets).
- Detects mouse button states (left, right, middle) and keyboard key presses.

### 4. Wayland Protocol Server ([`src/wayland`](src/wayland))
- Binds a native Wayland UNIX listening socket (e.g. `wayland-1`, `wayland-2`) under `$XDG_RUNTIME_DIR`.
- Implements core Wayland protocol globals:
  - `wl_compositor` (surface & region creation)
  - `wl_shm` & `wl_shm_pool` (shared memory client pixel transport supporting `ARGB8888` and `XRGB8888`)

### 5. Robust Logging Subsystem ([`src/logger.rs`](src/logger.rs))
- Unified `log_info!` macro that writes thread-safely to standard output and automatically flushes to `console.txt`.
- Built-in panic hook ensuring all runtime errors are captured to disk before termination.

---

## Project Structure

```
ascend/
├── Cargo.toml                  # Project manifest and dependencies
├── console.txt                 # Runtime log output (auto-generated)
├── src/
│   ├── main.rs                 # Application entry point & initialization sequence
│   ├── logger.rs               # Dual stdout + file logging engine with panic hook
│   ├── event_loop.rs           # Core non-blocking event loop
│   ├── drm/                    # Direct Rendering Manager & KMS layer
│   │   ├── mod.rs
│   │   ├── card.rs             # DRM device handle & master lock acquisition
│   │   ├── output.rs           # Connector, encoder, and CRTC discovery
│   │   ├── buffer.rs           # Legacy dumb buffer utilities
│   │   ├── flip.rs             # Hardware page-flip event listener
│   │   └── cursor.rs           # Dedicated DRM hardware cursor plane (ARGB8888)
│   ├── gbm/                    # Generic Buffer Management
│   │   ├── mod.rs
│   │   ├── device.rs           # GBM device initialization
│   │   └── buffer.rs           # DMA-BUF allocation & KMS framebuffer registration
│   ├── input/                  # Input device management via libinput
│   │   └── mod.rs              # udev seat assignment, pointer, and keyboard handling
│   ├── render/                 # Compositor rendering backends
│   │   ├── mod.rs              # Renderer trait & WindowSurface definition
│   │   ├── cpu.rs              # Fallback CPU software rasterizer
│   │   └── vulkan/             # GPU-accelerated Vulkan backend
│   │       ├── mod.rs
│   │       ├── context.rs      # Vulkan instance, physical device, & queue setup
│   │       ├── image.rs        # External DMA-BUF import to vk::Image
│   │       └── renderer.rs     # Command buffer recording, clears, & window blits
│   └── wayland/                # Wayland protocol server implementation
│       ├── mod.rs              # Server state, socket binding, client dispatch
│       ├── shm.rs              # Shared memory client buffer handlers
│       └── xdg_shell.rs        # XDG Shell surface management (in progress)
```

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

4. To stop the compositor, press **`Ctrl + C`**. All logs will be saved to `console.txt`.
