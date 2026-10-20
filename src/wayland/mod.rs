use anyhow::{Context, Result};
use std::ffi::OsString;
use std::sync::Arc;

use wayland_server::{
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, ListeningSocket, New,
    Resource, WEnum,
    backend::ClientData,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_compositor::{self, WlCompositor},
        wl_region::{self, WlRegion},
        wl_shm::{self, WlShm},
        wl_shm_pool::{self, WlShmPool},
        wl_surface::{self, WlSurface},
    },
};

struct AscendClientData;
impl ClientData for AscendClientData {
    fn initialized(&self, _client_id: wayland_server::backend::ClientId) {}
    fn disconnected(
        &self,
        _client_id: wayland_server::backend::ClientId,
        _reason: wayland_server::backend::DisconnectReason,
    ) {
    }
}

pub struct ServerState {
    pub surface: Vec<SurfaceData>,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            surface: Vec::new(),
        }
    }
}

impl GlobalDispatch<WlCompositor, ()> for ServerState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<WlCompositor>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl Dispatch<WlCompositor, ()> for ServerState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlCompositor,
        request: wl_compositor::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                println!("Wayland Client: Create a new surface");
                data_init.init(id, ());
            }

            wl_compositor::Request::CreateRegion { id } => {
                data_init.init(id, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<WlSurface, ()> for ServerState {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &WlSurface,
        request: wl_surface::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_surface::Request::Attach { buffer, x, y } => {
                println!(
                    "Wayland Client: Attached buffer {:?} at ({},{})",
                    buffer, x, y
                );

                let (width, height) = if let Some(buf) = &buffer {
                    if let Some(buf_data) = buf.data::<ShmBufferData>() {
                        println!(
                            "   -> Extracted SHM pixel buffer! Dimensions: {}x{}, Stride: {}, Raw bytes: {}",
                            buf_data.width,
                            buf_data.height,
                            buf_data.stride,
                            buf_data.pixel_data().len()
                        );
                        (buf_data.width, buf_data.height)
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                };

                if let Some(surf) = state.surface.iter_mut().find(|s| &s.surface == resource) {
                    surf.buffer = buffer;
                    surf.x = x;
                    surf.y = y;
                    surf.width = width;
                    surf.height = height;
                } else {
                    state.surface.push(SurfaceData {
                        surface: resource.clone(),
                        buffer,
                        x,
                        y,
                        width,
                        height,
                    });
                }
            }
            wl_surface::Request::Commit => {
                println!(
                    "Wayland Client: Committed surface! Total surfaces: {}",
                    state.surface.len()
                );
            }
            _ => {}
        }
    }
}

impl Dispatch<WlRegion, ()> for ServerState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlRegion,
        _request: wl_region::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}

// ============================================================================
// Shared Memory (SHM) Pool and Buffer storage
// ============================================================================

pub struct ShmPool {
    pub mmap: memmap2::Mmap,
}

#[derive(Clone)]
pub struct ShmBufferData {
    pub pool: Arc<ShmPool>,
    pub offset: usize,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: WEnum<wl_shm::Format>,
}

impl ShmBufferData {
    pub fn pixel_data(&self) -> &[u8] {
        let start = self.offset;
        let len = (self.stride * self.height) as usize;
        &self.pool.mmap[start..start + len]
    }
}

// ============================================================================
// wl_shm implementation (Shared Memory)
// ============================================================================

impl GlobalDispatch<WlShm, ()> for ServerState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<WlShm>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let shm = data_init.init(resource, ());

        shm.format(wl_shm::Format::Argb8888);
        shm.format(wl_shm::Format::Xrgb8888);
    }
}

impl Dispatch<WlShm, ()> for ServerState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlShm,
        request: wl_shm::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_shm::Request::CreatePool { id, fd, size } => {
                println!("Wayland Client: Created SHM pool with size {} bytes", size);

                match unsafe { memmap2::MmapOptions::new().len(size as usize).map(&fd) } {
                    Ok(mmap) => {
                        let pool = Arc::new(ShmPool { mmap });
                        data_init.init(id, pool);
                    }
                    Err(err) => {
                        eprintln!("Failed to mmap SHM pool: {}", err);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlShmPool, Arc<ShmPool>> for ServerState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlShmPool,
        request: wl_shm_pool::Request,
        data: &Arc<ShmPool>,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_shm_pool::Request::CreateBuffer {
                id,
                offset,
                width,
                height,
                stride,
                format,
            } => {
                println!(
                    "Wayland Client: Created buffer {}x{}, stride: {}, format: {:?}",
                    width, height, stride, format
                );
                let buffer_data = ShmBufferData {
                    pool: data.clone(),
                    offset: offset as usize,
                    width,
                    height,
                    stride,
                    format,
                };
                data_init.init(id, buffer_data);
            }
            _ => {}
        }
    }
}

impl Dispatch<WlBuffer, ShmBufferData> for ServerState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlBuffer,
        _request: wl_buffer::Request,
        _data: &ShmBufferData,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }
}

pub struct WaylandServer {
    pub display: Display<ServerState>,
    pub state: ServerState,
    pub socket: ListeningSocket,
    pub socket_name: OsString,
}

pub struct SurfaceData {
    pub surface: WlSurface,
    pub buffer: Option<WlBuffer>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WaylandServer {
    pub fn new() -> Result<Self> {
        let display = Display::<ServerState>::new().context("Failed to create Wayland display")?;
        let dh = display.handle();

        dh.create_global::<ServerState, WlCompositor, ()>(6, ());
        dh.create_global::<ServerState, WlShm, ()>(1, ());

        let socket = ListeningSocket::bind_auto("wayland", 1..32)
            .context("Failed to bind Wayland listening socket (check XDG_RUNTIME_DIR)")?;

        let socket_name = socket
            .socket_name()
            .map(|s| s.to_os_string())
            .unwrap_or_else(|| OsString::from("wayland-1"));

        println!("Wayland server listening on socket: {:?}", socket_name);

        Ok(Self {
            display,
            state: ServerState::new(),
            socket,
            socket_name,
        })
    }

    pub fn dispatch(&mut self) -> Result<()> {
        while let Some(stream) = self.socket.accept().context("Failed to accept client")? {
            println!("New Wayland client connected!");
            self.display
                .handle()
                .insert_client(stream, Arc::new(AscendClientData))
                .context("Failed to insert client into Wayland display")?;
        }

        self.display
            .dispatch_clients(&mut self.state)
            .context("Failed to dispatch Wayland clients")?;

        self.display
            .flush_clients()
            .context("Failed to flush Wayland clients")?;

        Ok(())
    }
}
