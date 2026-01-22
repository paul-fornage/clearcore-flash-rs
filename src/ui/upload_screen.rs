use std::path::PathBuf;
use std::time::Duration;
use iced::widget::{button, column, container, progress_bar, row, scrollable, text, Space};
use iced::{widget, Element, Length, Task, Theme};
use iced_selection::text as selectable_text;
use crate::app::App;
use crate::types::{AppScreen, LogEntry};
use crate::{serial, Message};
use crate::serial::upload::UploadEvent;
use crate::ui::common::logs_to_container;
use crate::ui::MainScreenMessage;

#[derive(Debug, Clone)]
pub enum UploadScreenMessage {
    Event(UploadEvent),
}

const UPLOAD_LOG_SCROLLABLE_ID: widget::Id = widget::Id::new("upload_log_scrollable_id");

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
            let range = 0f32..=prog_bar.total as f32;
            let current_progress = prog_bar.current as f32;
            let percent = current_progress / prog_bar.total as f32 * 100.0;
            container(column![
                text(prog_bar.phase.to_string()).size(32),
                row![
                    progress_bar(range, current_progress),
                    text(format!("{percent:>6.2}% ({}/{} pages)", prog_bar.current, prog_bar.total))
                        .size(16).font(iced::Font::MONOSPACE)
                ]
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

    let log_view_container = logs_to_container(&state.logs, &UPLOAD_LOG_SCROLLABLE_ID, !is_connected);

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