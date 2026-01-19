
use iced::widget::{button, checkbox, column, container, row, scrollable, stack, text, Space};
use iced::{alignment, Border, Color, Element, Length, Theme};
use iced_selection::text as selectable_text;

use crate::types::{LogEntry, Toast, ToastLevel, UploadProgress};
use crate::Message;




/// Render the upload screen with progress and logs
pub fn upload_screen(progress: &UploadProgress, logs: &[LogEntry]) -> Element<'static, Message> {
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

    let log_text = logs
        .iter()
        .map(|entry| format!("[{}] {}", entry.timestamp, entry.message))
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