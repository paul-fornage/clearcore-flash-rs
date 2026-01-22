use std::path::PathBuf;
use iced::widget::{button, column, container, progress_bar, row, text, Space};
use iced::{widget, Color, Element, Length, Task, Theme};
use crate::app::App;
use crate::types::{AppScreen, LogEntry};
use crate::Message;
use crate::serial::download::DownloadEvent;
use crate::ui::common::logs_to_container;

#[derive(Debug, Clone)]
pub enum DownloadScreenMessage {
    Event(DownloadEvent),
    SaveFileResult(Result<PathBuf, String>),
    SaveCancelled,
}

const DOWNLOAD_LOG_SCROLLABLE_ID: widget::Id = widget::Id::new("download_log_scrollable_id");

/// Download state and progress
#[derive(Debug, Clone, PartialEq)]
pub struct DownloadState {
    pub progress: DownloadProgress,
    pub logs: Vec<LogEntry>,
}

impl DownloadState {
    pub fn new() -> Self {
        Self {
            progress: DownloadProgress::Preparing,
            logs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadProgress {
    Preparing,
    Downloading(crate::serial::download::ProgressBar),
    Complete,
    Failed(String),
}

/// Render the download screen with progress and logs
pub fn download_screen(state: &DownloadState) -> Element<'static, Message> {
    let progress = &state.progress;

    let title = text("Downloading Firmware")
        .size(24)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        });

    let progress_text = match progress {
        DownloadProgress::Preparing => container(text("Preparing download...").size(16)),
        DownloadProgress::Downloading(prog_bar) => {
            let range = 0f32..=prog_bar.total as f32;
            let current_progress = prog_bar.current as f32;
            let percent = if prog_bar.total > 0 { current_progress / prog_bar.total as f32 * 100.0 } else { 0.0 };
            container(column![
                text(prog_bar.phase.to_string()).size(32),
                row![
                    progress_bar(range, current_progress),
                    text(format!("{percent:>6.2}% ({}/{} pages)", prog_bar.current, prog_bar.total))
                        .size(16).font(iced::Font::MONOSPACE)
                ]
            ])
        }
        DownloadProgress::Complete => container(
            text("Download complete!").size(16).style(|theme: &Theme| {
                text::Style { color: Some(theme.palette().success), } })
        ),
        DownloadProgress::Failed(err) => container(
            text(format!("Download failed: {}", err))
                .size(16)
                .style(|theme: &Theme| text::Style {
                    color: Some(theme.palette().danger),
                })
        ),
    };

    let color_override = match state.progress {
        DownloadProgress::Complete => { Some(Color::from_rgb(0.0, 1.0, 0.0)) }
        DownloadProgress::Failed(_) => { Some(Color::from_rgb(1.0, 0.0, 0.0)) }
        _ => None,
    };

    let log_view_container = logs_to_container(&state.logs, &DOWNLOAD_LOG_SCROLLABLE_ID, color_override);

    let back_button = if matches!(progress, DownloadProgress::Complete | DownloadProgress::Failed(_)) {
        Some(
            button(text("← Back to Main").size(16))
                .on_press(Message::BackToMain)
                .padding(8),
        )
    } else {
        None
    };

    let mut content = column![
        title, 
        Space::new().height(10), 
        progress_text, 
        Space::new().height(20), 
        log_view_container
    ]
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

impl App {
    pub fn handle_download_screen_message(&mut self, msg: DownloadScreenMessage) -> Task<Message> {
        match msg {
            DownloadScreenMessage::Event(event) => {
                if let AppScreen::Download(ref mut state) = self.screen {
                    match event {
                        DownloadEvent::Log(line) => {
                            state.logs.push(LogEntry::new_now(line));
                        }
                        DownloadEvent::Error(err) => {
                            state.progress = DownloadProgress::Failed(err.clone());
                            state.logs.push(LogEntry::new_now(format!("CRITICAL ERROR: {}", err)));
                        }
                        DownloadEvent::ProgressBarUpdate(progress) => {
                            state.progress = DownloadProgress::Downloading(progress);
                        }
                        DownloadEvent::Success => {
                            state.progress = DownloadProgress::Complete;
                            state.logs.push(LogEntry::new_now("SUCCESS: Firmware downloaded to temp storage. Prompting for save..."));

                            // Trigger the Save File Dialog immediately upon success
                            return Task::perform(
                                async {
                                    let file = rfd::AsyncFileDialog::new()
                                        .set_title("Save Firmware As...")
                                        .set_file_name("firmware.bin")
                                        .add_filter("Binary Files", &["bin"])
                                        .save_file()
                                        .await;

                                    if let Some(handle) = file {
                                        let dest_path = handle.path().to_path_buf();
                                        let temp_path = crate::serial::download::get_temp_download_path();

                                        // Perform the copy and delete
                                        match std::fs::copy(&temp_path, &dest_path) {
                                            Ok(_) => {
                                                let _ = std::fs::remove_file(temp_path);
                                                Ok(dest_path)
                                            },
                                            Err(e) => Err(format!("Failed to copy file: {}", e))
                                        }
                                    } else {
                                        // User cancelled
                                        Err("cancelled".to_string())
                                    }
                                },
                                |res| match res {
                                    Ok(path) => Message::DownloadScreen(DownloadScreenMessage::SaveFileResult(Ok(path))),
                                    Err(e) if e == "cancelled" => Message::DownloadScreen(DownloadScreenMessage::SaveCancelled),
                                    Err(e) => Message::DownloadScreen(DownloadScreenMessage::SaveFileResult(Err(e))),
                                }
                            );
                        }
                    }
                }
            }
            DownloadScreenMessage::SaveFileResult(result) => {
                match result {
                    Ok(path) => {
                        self.toast = Some(crate::ui::toast::Toast::info(format!("Saved to {:?}", path.file_name().unwrap_or_default())));
                        if let AppScreen::Download(state) = &mut self.screen {
                            state.logs.push(LogEntry::new_now(format!("Saved firmware to {:?}", path)));
                        }
                    }
                    Err(err) => {
                        self.toast = Some(crate::ui::toast::Toast::error(&err));
                        if let AppScreen::Download(state) = &mut self.screen {
                            state.logs.push(LogEntry::new_now(format!("Error saving file: {}", err)));
                        }
                    }
                }
            }
            DownloadScreenMessage::SaveCancelled => {
                self.toast = Some(crate::ui::toast::Toast::warning("File save cancelled"));
                if let AppScreen::Download(state) = &mut self.screen {
                    state.logs.push(LogEntry::new_now("User cancelled file save operation."));
                }
            }
        }
        Task::none()
    }
}