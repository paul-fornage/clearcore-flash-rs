use std::path::PathBuf;
use std::time::Duration;
use iced::widget::{button, column, container, row, stack, text, Container, Space};
use iced::{widget, Color, Element, Length, Renderer, Task, Theme};
use crate::app::App;
use crate::types::{AppScreen, LogEntry};
use crate::Message;
use crate::serial::upload::{UploadEvent, UploadProgressBar};
use crate::ui::common::{logs_to_container, prog_bar};
use crate::ui::MainScreenMessage;

#[derive(Debug, Clone)]
pub enum UploadScreenMessage {
    Event(UploadEvent),
}

const UPLOAD_LOG_SCROLLABLE_ID: widget::Id = widget::Id::new("upload_log_scrollable_id");

/// Upload state and progress
#[derive(Debug, Clone)]
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
    Uploading(UploadProgressBar),
    Complete,
    Failed(String),
}

impl UploadProgressBar {
    pub fn as_gui_element(&self) -> Container<'static, Message, Theme, Renderer> {
        prog_bar(self.total, self.current, &self.phase.to_string())
    }
}

/// Render the upload screen with progress and logs
pub fn upload_screen(state: &UploadState) -> Element<'static, Message> {
    let progress = &state.progress;

    let title = text("Uploading Firmware")
        .size(24)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        });

    let back_button = if matches!(progress, UploadProgress::Complete | UploadProgress::Failed(_)) {
        Some(
            button(text("← Back to Main").size(16))
                .on_press(Message::BackToMain)
                .padding(8),
        )
    } else {
        None
    };

    let header = match back_button {
        Some(back_button) => {
            container(stack![
                container(title)
                    .width(Length::Fill)
                    .align_x(iced::Alignment::Center),
                row![back_button]
                    .align_y(iced::Alignment::Center)
            ])
        },
        None => {
            container(title)
                .width(Length::Fill)
                .align_x(iced::Alignment::Center)
        }
    };

    let progress_text = match progress {
        UploadProgress::Preparing => container(text("Preparing upload...").size(16)),
        UploadProgress::Uploading ( prog_bar ) => {
            prog_bar.as_gui_element()
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

    let color_override = match state.progress {
        UploadProgress::Complete => { Some(Color::from_rgb(0.0, 1.0, 0.0)) }
        UploadProgress::Failed(_) => { Some(Color::from_rgb(1.0, 0.0, 0.0)) }
        _ => None,
    };
    let log_view_container = logs_to_container(&state.logs, &UPLOAD_LOG_SCROLLABLE_ID, color_override);



    let content = column![
        header,
        Space::new().height(10),
        progress_text,
        Space::new().height(20),
        log_view_container
    ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);


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
                            state.logs.push(LogEntry::new_error_now(format!("CRITICAL ERROR: {}", err)));
                        }
                        UploadEvent::ProgressBarUpdate(progress) => {
                            state.progress = UploadProgress::Uploading(progress);
                        }
                        UploadEvent::Success => {
                            state.progress = UploadProgress::Complete;
                            state.logs.push(LogEntry::new_info_now("SUCCESS: Firmware uploaded successfully."));

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