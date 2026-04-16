use futures::SinkExt;
use std::fmt::Debug;
use std::time::Duration;
use crate::types::{LogMsg, LogMsgType, UsbId};
use anyhow::{Context, Result};
use iced::futures::channel::mpsc;
use tokio::time::timeout;
use tokio_serial::{SerialPortBuilderExt, SerialPortInfo, SerialPortType};

pub mod monitor;
pub mod upload;
pub mod download;






pub const TEKNIC_BOOTLOADER_OFFSET_ADDRESS: u32 = 0x4000;

#[macro_export]
macro_rules! log_send {
    ($output:expr, $level:ident, $($arg:tt)+) => {{
        let msg = format!($($arg)+);

        match LogMsgType::$level {
            LogMsgType::Error => log::error!("{}", msg),
            LogMsgType::Warn => log::warn!("{}", msg),
            LogMsgType::Info => log::info!("{}", msg),
            LogMsgType::Debug => log::debug!("{}", msg),
            LogMsgType::Trace => log::trace!("{}", msg),
            _ => eprint!("{}", msg),
        }

        log_minor_err($output.send(LogMsg::new(LogMsgType::$level, msg).into()).await);
    }};
}

#[macro_export]
macro_rules! log_send_blocking {
    ($output:expr, $level:ident, $($arg:tt)+) => {{
        let msg = format!($($arg)+);

        match LogMsgType::$level {
            LogMsgType::Error => log::error!("{}", msg),
            LogMsgType::Warn => log::warn!("{}", msg),
            LogMsgType::Info => log::info!("{}", msg),
            LogMsgType::Debug => log::debug!("{}", msg),
            LogMsgType::Trace => log::trace!("{}", msg),
            _ => eprint!("{}", msg),
        }

        log_minor_err($output.send(LogMsg::new(LogMsgType::$level, msg).into()));
    }};
}


/// Helper to find port asynchronously.
/// Uses spawn_blocking because enumerating OS ports is a blocking operation.
async fn find_port_async(config: UsbId) -> Result<SerialPortInfo> {

    tokio::task::spawn_blocking(move || {
        let ports = tokio_serial::available_ports().context("Failed to list available ports")?;
        
        ports.into_iter().find(|port| {
            match &port.port_type {
                SerialPortType::UsbPort(usb_info) => {
                    usb_info.vid == config.vid && usb_info.pid == config.pid
                },
                _ => false
            }
        }).ok_or_else(|| {
            let vid = config.vid;
            let pid = config.pid;
            log::trace!("Failed to find port with VID/PID: {vid:04x}:{pid:04x}\n\
                    all ports: {:#?}", tokio_serial::available_ports());
            anyhow::anyhow!("Failed to find port with VID/PID: {vid:04x}:{pid:04x}")
        })
    }).await?
}


async fn touch_to_bootloader<T: From<LogMsg>>(output: &mut mpsc::Sender<T>, port_name: &str) -> Result<SerialPortInfo> {
    match tokio_serial::new(port_name, 1200).open_native_async(){
        Ok(cc_serial_port) => {
            log_send!(output, Info, "cc serial port {port_name} opened");

            drop(cc_serial_port);

            log_send!(output, Info, "cc serial port {port_name} closed");
        },
        Err(e) => {
            log_send!(output, Error, "Failed to open cc serial port {port_name} at 1200bps. \n\
            This might not be a problem on windows. Error: {e}");
        }
    }

    timeout(Duration::from_secs(5), wait_for_serial_port_disconnect(UsbId::CLEARCORE_SERIAL))
        .await.context(format!("Timed out waiting for cc serial port {port_name} disconnect"))?
        .context(format!("unexpected error while waiting for serial port {port_name} to disconnect after touching"))?;

    log_send!(output, Info, "cc serial port disconnected");

    let bootloader_port = timeout(
        Duration::from_secs(10), wait_for_serial_port(UsbId::CLEARCORE_BOOTLOADER)
    ).await.context("Timed out waiting for cc bootloader port")?
        .context("unexpected error while waiting for bootloader port to appear after touching")?;

    log::info!("cc bootloader found");

    Ok(bootloader_port)
}


pub async fn wait_for_serial_port(usb_id: UsbId) -> Result<SerialPortInfo> {
    loop {
        let ports = tokio_serial::available_ports()?;
        if let Some(port) = ports.iter().find(|&port| { is_specified_port(port, usb_id) }) {
            return Ok(port.clone());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}


pub async fn wait_for_serial_port_disconnect(usb_id: UsbId) -> Result<()> {
    loop {
        let ports = tokio_serial::available_ports()?;
        if ports.iter().any(|port| { is_specified_port(&port, usb_id) }) {
            tokio::time::sleep(Duration::from_millis(1)).await;
            continue;
        }
        return Ok(());
    }
}

pub fn is_specified_port(port_info: &SerialPortInfo, target_usb_id: UsbId) -> bool {
    match &port_info.port_type {
        SerialPortType::UsbPort(usb_info) => {
            let usb_id = UsbId::from(usb_info);
            usb_id == target_usb_id
        },
        _ => false
    }
}


fn log_minor_err<E: Debug>(res: Result<(), E>) {
    if let Err(err) = res {
        log::warn!("Upload stream warning: {:?}", err);
    }
}


async fn get_or_touch_to_bootloader<T: From<LogMsg>>(output: &mut mpsc::Sender<T>) -> Result<SerialPortInfo> {
    let bootloader_port_info = if let Ok(cc_serial_port_info) = find_port_async(UsbId::CLEARCORE_SERIAL).await{
        log_send!(output, Info, "cc serial port found");
        let port_name = cc_serial_port_info.port_name.clone();
        let cc_bootloader_port_info = touch_to_bootloader(output, &port_name).await
            .context("Failed to touch cc serial port to bootloader")?;
        log_send!(output, Info, "cc touched to bootloader, waiting 1 second");
        tokio::time::sleep(Duration::from_secs(1)).await;
        cc_bootloader_port_info
    } else {
        if let Ok(cc_bootloader_port_info) = find_port_async(UsbId::CLEARCORE_BOOTLOADER).await {
            log_send!(output, Warn, "Clearcore already in bootloader mode, skipping touching to bootloader");
            cc_bootloader_port_info
        } else {
            log_send!(output, Error, "Failed to find cc serial port");
            anyhow::bail!("Failed to find cc serial port");
        }
    };
    Ok(bootloader_port_info)
}
