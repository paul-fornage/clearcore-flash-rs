
use iced::widget::{button, checkbox, column, container, row, scrollable, stack, text, Space};
use iced::{alignment, Border, Color, Element, Length, Theme};
use iced_selection::text as selectable_text;

use crate::types::{LogEntry};
use crate::Message;

/// Toast notification
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToastLevel {
    Info,
    Warning,
    Error,
}

impl Toast {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Error,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Warning,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            level: ToastLevel::Info,
        }
    }
}



/// Render a toast notification
fn toast_widget(toast: &Toast) -> Element<'_, Message> {
    let (bg_color, text_color) = match toast.level {
        ToastLevel::Info => (Color::from_rgb(0.2, 0.4, 0.8), Color::WHITE),
        ToastLevel::Warning => (Color::from_rgb(0.8, 0.6, 0.2), Color::WHITE),
        ToastLevel::Error => (Color::from_rgb(0.8, 0.2, 0.2), Color::WHITE),
    };

    let close_button = button(text("×").size(24))
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
            .width(Length::Fill)
            .padding([10, 20])
            .align_x(alignment::Horizontal::Right)
            .align_y(alignment::Vertical::Bottom);

        stack![view, toast_overlay]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        view
    }
}