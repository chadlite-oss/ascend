use anyhow::{Context, Result, anyhow};
use drm::control::{Device as ControlDevice, Mode, connector, crtc};

use super::Card;

pub struct Output {
    pub connector: connector::Handle,
    pub crtc: crtc::Handle,
    pub mode: Mode,
    pub width: u32,
    pub height: u32,
}

impl Output {
    pub fn find_connected(card: &Card) -> Result<Self> {
        let resources = card.resources()?;

        for &conn_handle in resources.connectors() {
            let conn = card
                .get_connector(conn_handle, false)
                .context("Failed to query connector")?;

            if conn.state() != connector::State::Connected {
                continue;
            }

            let encoder_handle = conn
                .current_encoder()
                .ok_or_else(|| anyhow!("Connected connector has no encoder"))?;

            let encoder = card
                .get_encoder(encoder_handle)
                .context("Failed to query encoder")?;

            let crtc_handle = encoder
                .crtc()
                .ok_or_else(|| anyhow!("Encoder has no CRTC assigned"))?;

            let mode = *conn
                .modes()
                .first()
                .ok_or_else(|| anyhow!("Connector has modes"))?;

            let (width, height) = mode.size();

            return Ok(Self {
                connector: conn_handle,
                crtc: crtc_handle,
                mode,
                width: width as u32,
                height: height as u32,
            });
        }

        Err(anyhow!("No connected display found"))
    }

    pub fn describe(&self) {
        println!(
            "Output: connector {:?}, CRTC {:?}, {}x{} @ {}Hz",
            self.connector,
            self.crtc,
            self.width,
            self.height,
            self.mode.vrefresh()
        );
    }
}
