use std::time::Duration;
use iced::widget::{button, column, container, scrollable, text, Space};
use iced::{Element, Length, Task, Theme};
use iced_selection::text as selectable_text;
use crate::app::App;
use crate::types::{AppScreen, LogEntry, UploadProgress};
use crate::Message;
use crate::ui::MainScreenMessage;

#[derive(Debug, Clone)]
pub enum UploadScreenMessage {
    UploadProgress(UploadProgress),
}

/// Render the upload screen with progress and logs
pub fn upload_screen(progress: &UploadProgress) -> Element<'static, Message> {
    let title = text("Uploading Firmware")
        .size(24)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        });

    let progress_text = match progress {
        UploadProgress::Preparing => text("Preparing upload...").size(16),
        UploadProgress::Uploading { percent } => {
            text(format!("Uploading: {:.1}%", percent)).size(16)
        }
        UploadProgress::Complete => text("Upload complete!").size(16).style(|theme: &Theme| {
            text::Style {
                color: Some(theme.palette().success),
            }
        }),
        UploadProgress::Failed(err) => text(format!("Upload failed: {}", err))
            .size(16)
            .style(|theme: &Theme| text::Style {
                color: Some(theme.palette().danger),
            }),
    };
    

    let log_view = scrollable(
        container(
            selectable_text("log_text")
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
            UploadScreenMessage::UploadProgress(progress) => {
                if let AppScreen::Upload(ref mut state) = self.screen {
                    let monitor_after = state.monitor_after;
                    state.progress = progress.clone();

                    if let UploadProgress::Complete = progress {

                        if monitor_after {
                            // Transition to monitor screen
                            return Task::perform(
                                async {
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                },
                                |_| Message::MainScreen(MainScreenMessage::StartMonitoring),
                            );
                        }
                    } else if let UploadProgress::Failed(ref err) = progress {
                        log::error!("Upload failed: {}", err);
                    }
                }
            }
        }
        Task::none()
    }
}