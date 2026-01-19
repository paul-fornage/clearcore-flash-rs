use iced::{Element, Task, Theme};
use serialport::SerialPort;
use std::path::PathBuf;
use std::time::Duration;

use crate::serial;
use crate::types::{AppScreen, LogEntry, SerialConfig, Toast, UploadProgress, UploadState};
use crate::ui;

/// Main application state
pub struct App {
    screen: AppScreen,
    monitor_after_upload: bool,
    serial_config: SerialConfig,
    serial_port: Option<Box<dyn SerialPort>>,
    logs: Vec<LogEntry>,
    toast: Option<Toast>,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    // Main screen
    SelectFile,
    FileSelected(Option<PathBuf>),
    ToggleMonitorAfterUpload(bool),
    StartMonitoring,

    // Upload screen
    UploadProgress(UploadProgress),
    UploadLog(String),

    // Monitor screen
    SerialData(Vec<u8>),
    SerialError(String),
    BackToMain,

    // Toast
    CloseToast,

    // Periodic tick for reading serial data
    Tick,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                screen: AppScreen::Main,
                monitor_after_upload: false,
                serial_config: SerialConfig::default(),
                serial_port: None,
                logs: Vec::new(),
                toast: None,
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        match self.screen {
            AppScreen::Main => "ClearCore Flasher".to_string(),
            AppScreen::Upload(_) => "ClearCore Flasher - Uploading".to_string(),
            AppScreen::Monitor => "ClearCore Flasher - Monitor".to_string(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectFile => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Binary Files", &["bin"])
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    Message::FileSelected,
                );
            }

            Message::FileSelected(Some(path)) => {
                log::info!("Selected file: {:?}", path);
                self.logs.clear();
                self.logs.push(LogEntry::new(format!(
                    "Selected file: {}",
                    path.display()
                )));

                self.screen = AppScreen::Upload(UploadState {
                    file_path: path.clone(),
                    progress: UploadProgress::Preparing,
                    monitor_after: self.monitor_after_upload,
                });

                // Start the upload process
                return Task::perform(
                    upload_firmware(path),
                    |_| Message::UploadProgress(UploadProgress::Complete),
                );
            }

            Message::FileSelected(None) => {
                log::info!("File selection cancelled");
            }

            Message::ToggleMonitorAfterUpload(enabled) => {
                self.monitor_after_upload = enabled;
            }

            Message::StartMonitoring => {
                self.logs.clear();
                match self.open_monitor() {
                    Ok(_) => {
                        self.screen = AppScreen::Monitor;
                        return self.start_serial_reading();
                    }
                    Err(e) => {
                        log::error!("Failed to start monitoring: {}", e);
                        self.toast = Some(Toast::error(e.to_string()));

                    }
                }
            }

            Message::UploadProgress(progress) => {
                if let AppScreen::Upload(ref mut state) = self.screen {
                    let monitor_after = state.monitor_after;
                    state.progress = progress.clone();

                    if let UploadProgress::Complete = progress {
                        self.logs.push(LogEntry::new("Upload complete!".to_string()));

                        if monitor_after {
                            // Transition to monitor screen
                            return Task::perform(
                                async {
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                },
                                |_| Message::StartMonitoring,
                            );
                        }
                    } else if let UploadProgress::Failed(ref err) = progress {
                        self.logs.push(LogEntry::new(format!("Upload failed: {}", err)));
                    }
                }
            }

            Message::UploadLog(log_msg) => {
                self.logs.push(LogEntry::new(log_msg));
            }

            Message::SerialData(data) => {
                if let Ok(text) = String::from_utf8(data) {
                    for line in text.lines() {
                        self.logs.push(LogEntry::new(line.to_string()));
                    }
                }
                return self.start_serial_reading();
            }

            Message::SerialError(err) => {
                self.toast = Some(Toast::error(err.clone()));
                log::error!("Serial error: {}", err);
                self.serial_port = None;
            }

            Message::BackToMain => {
                // Close serial port if open
                self.serial_port = None;
                self.screen = AppScreen::Main;
                self.logs.clear();
                self.toast = None;
            }

            Message::CloseToast => {
                self.toast = None;
            }

            Message::Tick => {
                // Handled by serial reading task
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let view = match &self.screen {
            AppScreen::Main => ui::main_screen(self.monitor_after_upload),
            AppScreen::Upload(state) => ui::upload_screen(&state.progress, &self.logs),
            AppScreen::Monitor => ui::monitor_screen(&self.logs),
        };

        ui::with_toast(view, self.toast.as_ref())
    }

    pub fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }

    fn open_monitor(&mut self) -> anyhow::Result<()> {
        let port_name = serial::find_clearcore_port(&self.serial_config)?;
        let port = serial::open_serial_port(&port_name, &self.serial_config)?;
        self.serial_port = Some(port);
        self.logs.push(LogEntry::new(format!("Connected to {}", port_name)));
        Ok(())
    }

    fn start_serial_reading(&mut self) -> Task<Message> {
        if let Some(ref mut port) = self.serial_port {
            let mut port_clone = port.try_clone().expect("Failed to clone serial port");
            return Task::perform(
                async move {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    match serial::read_serial_data(&mut port_clone) {
                        Ok(data) => Message::SerialData(data),
                        Err(e) => Message::SerialError(e.to_string()),
                    }
                },
                |msg| msg,
            );
        }
        Task::none()
    }
}

/// Upload firmware to ClearCore (placeholder)
async fn upload_firmware(_path: PathBuf) -> anyhow::Result<()> {
    // Simulate upload process
    tokio::time::sleep(Duration::from_secs(2)).await;

    // TODO: Implement actual upload using bossac wrapper
    todo!("Implement firmware upload using bossac")
}
