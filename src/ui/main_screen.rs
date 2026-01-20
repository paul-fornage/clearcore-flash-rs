use iced::widget::{button, checkbox, column, container, row, text, Space};
use iced::{Element, Length, Task, Theme};

use crate::{serial, ui, Message};
use crate::app::{upload_firmware, App};
use crate::types::{AppScreen, LogEntry, SerialConfig, Toast, UploadProgress, UploadState};
use crate::ui::monitor_screen::MonitorState;

#[derive(Debug, Clone)]
pub enum MainScreenMessage {
    SelectFile,
    FileSelected(Option<std::path::PathBuf>),
    SetMonitorAfterUpload(bool),
    StartMonitoring,
    MonitorConnected,
    MonitorConnectionFailed(String),
}

/// Render the main screen with upload and monitor buttons
pub fn main_screen(monitor_after_upload: bool) -> Element<'static, Message> {
    let title = text("ClearCore Flasher")
        .size(32)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        });

    let upload_section = column![
        row![
            button(
                text("Upload Firmware")
                    .size(18)
                    .width(Length::Fill)
                    .center()
            )
            .on_press(Message::MainScreen(MainScreenMessage::SelectFile))
            .padding(16)
            .width(Length::Fixed(200.0)),
        ]
        .spacing(10),
        checkbox(monitor_after_upload)
            .label("Monitor after upload")
            .on_toggle(|enabled| Message::MainScreen(MainScreenMessage::SetMonitorAfterUpload(enabled)))
            .size(16),
    ]
        .spacing(12);

    let monitor_button = button(
        text("Monitor Serial")
            .size(18)
            .width(Length::Fill)
            .center()
    )
        .on_press(Message::MainScreen(MainScreenMessage::StartMonitoring))
        .padding(16)
        .width(Length::Fixed(200.0));

    let content = column![title, Space::new().height(30), upload_section, Space::new().height(20), monitor_button]
        .spacing(10)
        .padding(40)
        .width(Length::Fill)
        .align_x(iced::Alignment::Center);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

impl App{
    
    pub fn handle_main_screen_message(&mut self, msg: MainScreenMessage) -> Task<Message> {
        match msg {
            MainScreenMessage::SelectFile => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Binary Files", &["bin"])
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    |path| Message::MainScreen(MainScreenMessage::FileSelected(path)),
                );
            }

            MainScreenMessage::FileSelected(Some(path)) => {
                log::info!("Selected file: {:?}", path);

                self.screen = AppScreen::Upload(UploadState {
                    file_path: path.clone(),
                    progress: UploadProgress::Preparing,
                    monitor_after: self.monitor_after_upload,
                });

                // Start the upload process
                return Task::perform(
                    upload_firmware(path),
                    |_| Message::UploadScreen(ui::UploadScreenMessage::UploadProgress(UploadProgress::Complete)),
                );
            }

            MainScreenMessage::FileSelected(None) => {
                log::info!("File selection cancelled");
            }

            MainScreenMessage::SetMonitorAfterUpload(enabled) => {
                self.monitor_after_upload = enabled;
            }

            MainScreenMessage::StartMonitoring => {
                log::info!("Starting serial monitor...");
                self.toast = Some(Toast::info("Connecting...".to_string()));
                self.screen = AppScreen::Monitor(MonitorState::default());
            }

            MainScreenMessage::MonitorConnected => {
                // Not used anymore
            }

            MainScreenMessage::MonitorConnectionFailed(_e) => {
                // Not used anymore
            }
        }
        Task::none()
    }
}
