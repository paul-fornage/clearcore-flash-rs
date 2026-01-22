use iced::{Element, Task, Theme, Subscription};
use std::path::PathBuf;
use std::time::Duration;

use crate::serial;
use crate::serial::upload::UploadConfig;
use crate::types::{AppScreen};
use crate::ui;
use crate::ui::toast::Toast;

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

    DownloadScreen(ui::download_screen::DownloadScreenMessage),

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
            AppScreen::Upload(_) => "ClearCore Flasher - Upload".to_string(),
            AppScreen::Download(_) => "ClearCore Flasher - Download".to_string(),
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

            Message::DownloadScreen(msg) => {
                return self.handle_download_screen_message(msg);
            }

            Message::MonitorScreen(msg) => {
                return self.handle_monitor_screen_message(msg);
            }

            Message::BackToMain => {
                self.screen = AppScreen::Main;
                self.toast = None;
            }

            Message::CloseToast => {
                self.toast = None;
            }

            Message::Tick => {
                // Handled by subscription now
            }
        }

        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match &self.screen {
            // Only run subscription when on Monitor screen
            // The subscription itself handles the searching -> connecting flow
            AppScreen::Monitor(_) => {
                serial::monitor::listen().map(|event| match event {
                    serial::monitor::SerialMonitorEvent::Data(line) => {
                        Message::MonitorScreen(ui::monitor_screen::MonitorScreenMessage::SerialData(line))
                    }
                    serial::monitor::SerialMonitorEvent::StateChange(state) => {
                        Message::MonitorScreen(ui::monitor_screen::MonitorScreenMessage::ConnectionStateChanged(state))
                    }
                })
            }
            AppScreen::Upload(state) => {
                // Stop subscription if we are done or failed, to stop polling
                if matches!(state.progress, ui::upload_screen::UploadProgress::Complete | ui::upload_screen::UploadProgress::Failed(_)) {
                    Subscription::none()
                } else {
                    // Start the upload stream
                    let config = UploadConfig {
                        file_path: state.file_path.clone(),
                    };

                    serial::upload::listen(config).map(|event| {
                        Message::UploadScreen(ui::upload_screen::UploadScreenMessage::Event(event))
                    })
                }
            }
            AppScreen::Download(state) => {
                // Stop subscription if we are done or failed.
                // Note: Success state in download means file is in temp, we stop listening to serial events
                if matches!(state.progress, ui::download_screen::DownloadProgress::Complete | ui::download_screen::DownloadProgress::Failed(_)) {
                    Subscription::none()
                } else {
                    serial::download::listen().map(|event| {
                        Message::DownloadScreen(ui::download_screen::DownloadScreenMessage::Event(event))
                    })
                }
            }
            _ => Subscription::none(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let view = match &self.screen {
            AppScreen::Main => ui::main_screen(self.monitor_after_upload),
            AppScreen::Upload(state) => ui::upload_screen(state),
            AppScreen::Download(state) => ui::download_screen(state),
            AppScreen::Monitor(monitor_state) => ui::monitor_screen(monitor_state),
        };

        ui::with_toast(view, self.toast.as_ref())
    }

    pub fn theme(&self) -> Theme {
        Theme::Oxocarbon
    }
}