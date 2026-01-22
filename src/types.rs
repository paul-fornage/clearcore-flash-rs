use std::fmt::{Debug, Display};
use std::path::PathBuf;
use chrono::{DateTime, Local};
use tokio_serial::UsbPortInfo;
use crate::ui::download_screen::DownloadState;
use crate::ui::monitor_screen::MonitorState;
use crate::ui::upload_screen::UploadState;

/// Main application screens
#[derive(Debug)]
pub enum AppScreen {
    Main,
    Upload(UploadState),
    Monitor(MonitorState),
    Download(DownloadState),
}




#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UsbId {
    /// vendor ID
    pub vid: u16,
    /// product ID
    pub pid: u16,
}
impl Default for UsbId { fn default() -> Self { Self::CLEARCORE_SERIAL } }
impl UsbId{
    pub const CLEARCORE_SERIAL: Self = Self{vid: 0x2890, pid: 0x8022};
    pub const CLEARCORE_BOOTLOADER: Self = Self{vid: 0x2890, pid: 0x0022};
}
impl From<&UsbPortInfo> for UsbId{ fn from(usb_port_info: &UsbPortInfo) -> Self {
    Self { vid: usb_port_info.vid, pid: usb_port_info.pid }
} }
impl From<UsbPortInfo> for UsbId{
    fn from(usb_port_info: UsbPortInfo) -> Self { Self::from(&usb_port_info) }
}
impl Debug for UsbId{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UsbId({:04x}:{:04x})", self.vid, self.pid)
    }
}

/// Serial port configuration for ClearCore
#[derive(Debug, Clone, PartialEq)]
pub struct SerialConfig {
    pub usb_id: UsbId,
    pub baud_rate: u32,
}

impl SerialConfig {
    pub const SERIAL_MONITOR: Self = Self { usb_id: UsbId::CLEARCORE_SERIAL, baud_rate: 115200 };
    pub const BOOTLOADER_TOUCH: Self = Self { usb_id: UsbId::CLEARCORE_SERIAL, baud_rate: 1200 };
}

impl Default for SerialConfig { fn default() -> Self { Self::SERIAL_MONITOR } }

/// Log entry for displaying serial output
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub message: String,
}

impl LogEntry {
    pub fn new_now(message: impl Into<String>) -> Self {
        Self {
            timestamp: Local::now(),
            message: message.into(),
        }
    }
    pub fn new(timestamp: DateTime<Local>, message: String) -> Self {
        Self {
            timestamp,
            message,
        }
    }

    pub fn format_timestamp(&self) -> impl Display {
        self.timestamp.format("%H:%M:%S%.3f")
    }
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.format_timestamp(), self.message)
    }
}

