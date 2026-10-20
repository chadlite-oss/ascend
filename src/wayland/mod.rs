use anyhow::{Context, Result};
use std::ffi::OsString;
use std::sync::Arc;

use wayland_server::{
    Client, DataInit, Dispatch, Display, DisplayHandle, GlobalDispatch, ListeningSocket, New,
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

pub struct ServerState {}

impl ServerState {
    pub fn new() -> Self {
        Self {}
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
        _state: &mut Self,
        _client: &Client,
        _resource: &WlSurface,
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
            }
            wl_surface::Request::Commit => {
                println!("Wayland Client: Committed surface!");
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
            wl_shm::Request::CreatePool { id, fd: _, size } => {
                println!("Wayland Client: Created SHM pool with size {} bytes", size);

                data_init.init(id, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<WlShmPool, ()> for ServerState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlShmPool,
        request: wl_shm_pool::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_shm_pool::Request::CreateBuffer {
                id,
                offset: _,
                width,
                height,
                stride,
                format,
            } => {
                println!(
                    "Wayland Client: Created buffer {}x{},stride : {} , format:{:?}",
                    width, height, stride, format
                );
                data_init.init(id, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<WlBuffer, ()> for ServerState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &WlBuffer,
        _request: wl_buffer::Request,
        _data: &(),
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
