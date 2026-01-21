use std::path::PathBuf;
use std::time::Duration;
use iced::widget::{button, column, container, scrollable, text, Space};
use iced::{Element, Length, Task, Theme};
use iced_selection::text as selectable_text;
use crate::app::App;
use crate::types::{AppScreen, LogEntry};
use crate::{serial, Message};
use crate::serial::upload::UploadEvent;
use crate::ui::MainScreenMessage;

#[derive(Debug, Clone)]
pub enum UploadScreenMessage {
    Event(UploadEvent),
}

/// Upload state and progress
#[derive(Debug, Clone, PartialEq)]
pub struct UploadState {
    pub file_path: PathBuf,
    pub progress: UploadProgress,
    pub monitor_after: bool,
    pub logs: Vec<LogEntry>,
}

impl UploadState {
    pub fn new(file_path: PathBuf, monitor_after: bool) -> Self {
        Self {
            file_path,
            progress: UploadProgress::Preparing,
            monitor_after,
            logs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UploadProgress {
    Preparing,
    Uploading(serial::upload::ProgressBar),
    Complete,
    Failed(String),
}

/// Render the upload screen with progress and logs
pub fn upload_screen(state: &UploadState) -> Element<'static, Message> {
    let progress = &state.progress;

    let title = text("Uploading Firmware")
        .size(24)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        });

    let progress_text = match progress {
        UploadProgress::Preparing => container(text("Preparing upload...").size(16)),
        UploadProgress::Uploading ( prog_bar ) => {
            container(column![
                text(prog_bar.title.clone()).size(32),
                text(prog_bar.loading_bar_string()).size(16)
            ])
        }
        UploadProgress::Complete => container(
            text("Upload complete!").size(16).style(|theme: &Theme| {
                text::Style { color: Some(theme.palette().success), } })
        ),
        UploadProgress::Failed(err) => container(
            text(format!("Upload failed: {}", err))
                .size(16)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.palette().danger),
                })
        ),
    };

    let log_text = state.logs
        .iter()
        .map(|entry| format!("{entry}"))
        .collect::<Vec<_>>()
        .join("\n");

    let log_view = scrollable(
        container(
            selectable_text(log_text)
                .font(iced::Font::MONOSPACE)
                .size(14)
        )
            .padding(10)
            .width(Length::Fill)
    )
        .height(Length::Fill)
        .width(Length::Fill);

    let back_button = if matches!(progress, UploadProgress::Complete | UploadProgress::Failed(_)) {
        Some(
            button(text("← Back to Main").size(16))
                .on_press(Message::BackToMain)
                .padding(8),
        )
    } else {
        None
    };

    let mut content = column![title, Space::new().height(10), progress_text, Space::new().height(20), log_view]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

    if let Some(btn) = back_button {
        content = content.push(Space::new().height(10)).push(btn);
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}


impl App{
    pub fn handle_upload_screen_message(&mut self, msg: UploadScreenMessage) -> Task<Message> {
        match msg {
            UploadScreenMessage::Event(event) => {
                if let AppScreen::Upload(ref mut state) = self.screen {
                    match event {
                        UploadEvent::Log(line) => {
                            state.logs.push(LogEntry::new_now(line));
                        }
                        UploadEvent::Error(err) => {
                            state.progress = UploadProgress::Failed(err.clone());
                            state.logs.push(LogEntry::new_now(format!("CRITICAL ERROR: {}", err)));
                        }
                        UploadEvent::ProgressBarUpdate(progress) => {
                            state.progress = UploadProgress::Uploading(progress);
                        }
                        UploadEvent::Success => {
                            state.progress = UploadProgress::Complete;
                            state.logs.push(LogEntry::new_now("SUCCESS: Firmware uploaded successfully."));

                            if state.monitor_after {
                                // Transition to monitor screen automatically
                                return Task::perform(
                                    async {
                                        tokio::time::sleep(Duration::from_secs(1)).await;
                                    },
                                    |_| Message::MainScreen(MainScreenMessage::StartMonitoring),
                                );
                            }
                        }
                    }
                }
            }
        }
        Task::none()
    }
}