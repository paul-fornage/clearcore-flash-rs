use iced::widget::{button, checkbox, column, container, row, text, Space};
use iced::{Element, Length, Task, Theme};

use crate::{serial, ui, Message};
use crate::app::App;
use crate::types::{AppScreen, LogEntry, SerialConfig};
use crate::ui::common::card;
use crate::ui::monitor_screen::MonitorState;
use crate::ui::upload_screen::{UploadProgress, UploadState};
use crate::ui::download_screen::DownloadState;

#[derive(Debug, Clone)]
pub enum MainScreenMessage {
    SelectFile,
    FileSelected(Option<std::path::PathBuf>),
    SetMonitorAfterUpload(bool),
    StartMonitoring,
    StartDownload,
}

/// Render the main screen with upload and monitor buttons
pub fn main_screen(monitor_after_upload: bool) -> Element<'static, Message> {
    let card_height = Length::Fixed(120.0);
    let title = text("ClearCore Flasher")
        .size(32)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        });

    let upload_section = card(
        container(
            column![
                button(
                    container(
                        text("Upload Firmware")
                            .size(18)
                            .center()
                    )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                )
                .on_press(Message::MainScreen(MainScreenMessage::SelectFile))
                .padding(16)
                .width(Length::Fixed(240.0)),

                checkbox(monitor_after_upload)
                    .label("Monitor after upload")
                    .on_toggle(|enabled| {
                        Message::MainScreen(MainScreenMessage::SetMonitorAfterUpload(enabled))
                    })
                    .size(16)
                    .width(Length::Fixed(240.0)),
            ]
                .spacing(12)
                .align_x(iced::Alignment::Center),
        )
            .width(Length::Fill)
            .center_x(Length::Fill),
    )
        .width(Length::Fill)
        .height(card_height);

    let download_button = card(
        container(
            button(
                container(
                    text("Download Firmware")
                        .size(18)
                        .center(),
                )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
                .on_press(Message::MainScreen(MainScreenMessage::StartDownload))
                .padding(16)
                .width(Length::Fixed(240.0))
                .height(Length::Fill),
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill),
    )
        .width(Length::Fill)
        .height(card_height);

    let monitor_button = card(
        container(
            button(
                container(
                    text("Monitor Serial")
                        .size(18)
                        .center(),
                )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
                .on_press(Message::MainScreen(MainScreenMessage::StartMonitoring))
                .padding(16)
                .width(Length::Fixed(240.0))
                .height(Length::Fill),
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill),
    )
        .width(Length::Fill)
        .height(card_height);

    let content = column![
        title,
        Space::new().height(30),
        row![
            upload_section,
            Space::new().width(10),
            download_button,
            Space::new().width(10),
            monitor_button
        ]
        .align_y(iced::Alignment::Start)
    ]
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
                    logs: Vec::new(),
                });
            }

            MainScreenMessage::FileSelected(None) => {
                log::info!("File selection cancelled");
            }

            MainScreenMessage::SetMonitorAfterUpload(enabled) => {
                self.monitor_after_upload = enabled;
            }

            MainScreenMessage::StartMonitoring => {
                log::info!("Starting serial monitor...");
                self.screen = AppScreen::Monitor(MonitorState::default());
            }

            MainScreenMessage::StartDownload => {
                log::info!("Starting firmware download...");
                self.screen = AppScreen::Download(DownloadState::new());
            }
        }
        Task::none()
    }
}
