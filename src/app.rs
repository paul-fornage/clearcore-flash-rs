use iced::{Element, Task, Theme};
use serialport::SerialPort;
use std::path::PathBuf;
use std::time::Duration;

use crate::serial;
use crate::types::{AppScreen, LogEntry, SerialConfig, Toast, UploadProgress, UploadState};
use crate::ui;

/// Main application state
pub struct App {
    pub screen: AppScreen,
    pub monitor_after_upload: bool,
    
    pub toast: Option<Toast>,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    // Main screen
    MainScreen(ui::main_screen::MainScreenMessage),

    // Upload screen
    UploadScreen(ui::upload_screen::UploadScreenMessage),

    // Monitor screen
    MonitorScreen(ui::monitor_screen::MonitorScreenMessage),

    // Global
    BackToMain,
    CloseToast,
    Tick,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                screen: AppScreen::Main,
                monitor_after_upload: false,
                toast: None,
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        match self.screen {
            AppScreen::Main => "ClearCore Flasher".to_string(),
            AppScreen::Upload(_) => "ClearCore Flasher - Uploading".to_string(),
            AppScreen::Monitor(_) => "ClearCore Flasher - Monitor".to_string(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainScreen(msg) => {
                return self.handle_main_screen_message(msg);
            }

            Message::UploadScreen(msg) => {
                return self.handle_upload_screen_message(msg);
            }

            Message::MonitorScreen(msg) => {
                return self.handle_monitor_screen_message(msg);
            }

            Message::BackToMain => {
                // Close serial port if open
                self.screen = AppScreen::Main;
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
            AppScreen::Upload(state) => ui::upload_screen(&state.progress),
            AppScreen::Monitor(monitor_state) => ui::monitor_screen(monitor_state),
        };

        ui::with_toast(view, self.toast.as_ref())
    }

    pub fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }

    pub(crate) fn open_monitor(&mut self) -> anyhow::Result<()> {
        let port_name = serial::find_clearcore_port(&self.serial_config)?;
        let port = serial::open_serial_port(&port_name, &self.serial_config)?;
        self.serial_port = Some(port);
        self.logs.push(LogEntry::new(format!("Connected to {}", port_name)));
        Ok(())
    }

    pub(crate) fn start_serial_reading(&mut self) -> Task<Message> {
        if let Some(ref mut port) = self.serial_port {
            let mut port_clone = port.try_clone().expect("Failed to clone serial port");
            return Task::perform(
                async move {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    match serial::read_serial_data(&mut port_clone) {
                        Ok(data) => Message::MonitorScreen(ui::MonitorScreenMessage::SerialData(data)),
                        Err(e) => Message::MonitorScreen(ui::MonitorScreenMessage::SerialError(e.to_string())),
                    }
                },
                |msg| msg,
            );
        }
        Task::none()
    }
}

/// Upload firmware to ClearCore (placeholder)
pub async fn upload_firmware(_path: PathBuf) -> anyhow::Result<()> {
    // Simulate upload process
    tokio::time::sleep(Duration::from_secs(2)).await;

    // TODO: Implement actual upload using bossac wrapper
    todo!("Implement firmware upload using bossac")
}
