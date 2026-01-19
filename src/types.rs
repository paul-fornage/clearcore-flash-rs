use std::path::PathBuf;

/// Main application screens
#[derive(Debug, Clone, PartialEq)]
pub enum AppScreen {
    Main,
    Upload(UploadState),
    Monitor,
}

/// Toast notification
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToastLevel {
    Info,
    Warning,
    Error,
}

impl Toast {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Error,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Warning,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Info,
        }
    }
}

/// Upload state and progress
#[derive(Debug, Clone, PartialEq)]
pub struct UploadState {
    pub file_path: PathBuf,
    pub progress: UploadProgress,
    pub monitor_after: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UploadProgress {
    Preparing,
    Uploading { percent: f32 },
    Complete,
    Failed(String),
}

/// Serial port configuration for ClearCore
#[derive(Debug, Clone)]
pub struct SerialConfig {
    pub vendor_id: u16,
    pub product_id: u16,
    pub baud_rate: u32,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            vendor_id: 0x2890,  // Microchip VID (typical for ClearCore)
            product_id: 0x8022, // ClearCore PID
            baud_rate: 115200,
        }
    }
}

/// Log entry for displaying serial output
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
}

impl LogEntry {
    pub fn new(message: String) -> Self {
        let now = chrono::Local::now();
        Self {
            timestamp: now.format("%H:%M:%S%.3f").to_string(),
            message,
        }
    }
}
