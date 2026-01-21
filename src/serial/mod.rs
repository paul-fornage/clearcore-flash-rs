use crate::types::UsbId;
use anyhow::{Context, Result};

pub mod monitor;
pub mod upload;

/// Helper to find port asynchronously.
/// Uses spawn_blocking because enumerating OS ports is a blocking operation.
async fn find_port_async(config: &UsbId) -> Result<String> {
    let vid = config.vid;
    let pid = config.pid;

    tokio::task::spawn_blocking(move || {
        let ports = tokio_serial::available_ports().context("Failed to list available ports")?;

        for port in ports {
            if let tokio_serial::SerialPortInfo {
                port_name,
                port_type: tokio_serial::SerialPortType::UsbPort(usb_info),
            } = port
            {
                if usb_info.vid == vid && usb_info.pid == pid {
                    return Ok(port_name);
                }
            }
        }
        anyhow::bail!("Failed to find port with VID/PID: {:04x}:{:04x}", vid, pid);
    }).await?
}