use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use chrono::{DateTime, Local};
use iced::Color;
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


#[derive(Debug, Clone)]
pub struct LogMsg {
    pub message: String,
    pub log_type: LogMsgType,
}
impl LogMsg {
    pub fn new_cc(message: impl Into<String>) -> Self {
        Self { message: message.into(), log_type: LogMsgType::ClearCore }
    }
    pub fn new_bossa(message: impl Into<String>) -> Self {
        Self { message: message.into(), log_type: LogMsgType::BossaNative }
    }
    pub fn new_trace(message: impl Into<String>) -> Self {
        Self { message: message.into(), log_type: LogMsgType::Trace }
    }
    pub fn new_debug(message: impl Into<String>) -> Self {
        Self { message: message.into(), log_type: LogMsgType::Debug }
    }
    pub fn new_info(message: impl Into<String>) -> Self {
        Self { message: message.into(), log_type: LogMsgType::Info }
    }
    pub fn new_warn(message: impl Into<String>) -> Self {
        Self { message: message.into(), log_type: LogMsgType::Warn }
    }
    pub fn new_error(message: impl Into<String>) -> Self {
        Self { message: message.into(), log_type: LogMsgType::Error }
    }
    pub fn new(log_type: LogMsgType, message: impl Into<String>) -> Self {
        Self { message: message.into(), log_type: log_type }
    }
}

impl Display for LogMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if matches!(self.log_type, LogMsgType::BossaNative) {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{:?}: {}", self.log_type, self.message)
        }
    }
}

impl Into<String> for LogMsg {
    fn into(self) -> String {
        format!("{}", self)
    }
}

#[derive(Debug, Clone)]
pub enum LogMsgType{
    BossaNative,
    ClearCore,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogMsgType{
    pub fn as_str(&self) -> &'static str {
        match self{
            LogMsgType::Trace => "TRACE: ",
            LogMsgType::Debug => "DEBUG: ",
            LogMsgType::Info => "INFO: ",
            LogMsgType::Warn => "WARN: ",
            LogMsgType::Error => "ERROR: ",
            _ => ""
        }
    }
}


/// Log entry for displaying serial output
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub message: LogMsg,
}

impl LogEntry {
    pub fn new_error_now(message: impl Into<String>) -> Self {
        Self::new_now(LogMsg::new_error(message))
    }
    pub fn new_warn_now(message: impl Into<String>) -> Self {
        Self::new_now(LogMsg::new_warn(message))
    }
    pub fn new_info_now(message: impl Into<String>) -> Self {
        Self::new_now(LogMsg::new_info(message))
    }
    pub fn new_debug_now(message: impl Into<String>) -> Self {
        Self::new_now(LogMsg::new_debug(message))
    }
    pub fn new_trace_now(message: impl Into<String>) -> Self {
        Self::new_now(LogMsg::new_trace(message))
    }
    pub fn new_bossa_now(message: impl Into<String>) -> Self {
        Self::new_now(LogMsg::new_bossa(message))
    }
    pub fn new_cc_now(message: impl Into<String>) -> Self {
        Self::new_now(LogMsg::new_cc(message))
    }
    pub fn new_now(message: LogMsg) -> Self {
        Self {
            timestamp: Local::now(),
            message: message.into(),
        }
    }
    pub fn new(timestamp: DateTime<Local>, message: LogMsg) -> Self {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}{}",
            self.format_timestamp(),
            self.message.log_type.as_str(),
            self.message.message.trim())
    }
}


