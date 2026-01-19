use iced::widget::{button, checkbox, column, container, row, scrollable, stack, text, Space};
use iced::{alignment, Border, Color, Element, Length, Theme};
use iced_selection::text as selectable_text;

use crate::types::{LogEntry, Toast, ToastLevel, UploadProgress};
use crate::Message;

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
            .on_press(Message::SelectFile)
            .padding(16)
            .width(Length::Fixed(200.0)),
        ]
        .spacing(10),
        checkbox(monitor_after_upload)
            .label("Monitor after upload")
            .on_toggle(Message::ToggleMonitorAfterUpload)
            .size(16),
    ]
    .spacing(12);

    let monitor_button = button(
        text("Monitor Serial")
            .size(18)
            .width(Length::Fill)
            .center()
    )
    .on_press(Message::StartMonitoring)
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

/// Render the monitor screen with serial log view
pub fn monitor_screen(logs: &[LogEntry]) -> Element<'static, Message> {
    let title = text("Serial Monitor")
        .size(24)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        });

    let back_button = button(text("← Back").size(16))
        .on_press(Message::BackToMain)
        .padding(8);

    let header = row![back_button, Space::new().width(20), title]
        .spacing(10)
        .align_y(iced::Alignment::Center);

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

    let content = column![header, log_view]
        .spacing(20)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

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

/// Render a toast notification
fn toast_widget(toast: &Toast) -> Element<'_, Message> {
    let (bg_color, text_color) = match toast.level {
        ToastLevel::Info => (Color::from_rgb(0.2, 0.4, 0.8), Color::WHITE),
        ToastLevel::Warning => (Color::from_rgb(0.8, 0.6, 0.2), Color::WHITE),
        ToastLevel::Error => (Color::from_rgb(0.8, 0.2, 0.2), Color::WHITE),
    };

    let close_button = button(text("×").size(18))
        .on_press(Message::CloseToast)
        .padding(4)
        .style(move |_theme: &Theme, _status| {
            let base = button::Style::default();
            button::Style {
                background: Some(iced::Background::Color(Color::TRANSPARENT)),
                text_color: text_color,
                border: Border::default(),
                ..base
            }
        });

    let content = row![
        text(&toast.message).size(14).style(move |_: &Theme| text::Style {
            color: Some(text_color),
        }),
        Space::new().width(10),
        close_button,
    ]
    .padding(12)
    .spacing(10)
    .align_y(iced::Alignment::Center);

    container(content)
        .style(move |_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(bg_color)),
            border: Border {
                color: Color::from_rgb(0.1, 0.1, 0.1),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Wrap view with optional toast overlay
pub fn with_toast<'a>(view: Element<'a, Message>, toast: Option<&'a Toast>) -> Element<'a, Message> {
    if let Some(t) = toast {
        let toast_overlay = container(toast_widget(t))
            .width(Length::Shrink)
            .padding([10, 20])
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Bottom);

        stack![view, toast_overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        view
    }
}
