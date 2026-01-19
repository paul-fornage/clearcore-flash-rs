
use iced::widget::{button, checkbox, column, container, row, scrollable, stack, text, Space};
use iced::{alignment, Border, Color, Element, Length, Theme};
use iced_selection::text as selectable_text;

use crate::types::{LogEntry, Toast, ToastLevel, UploadProgress};
use crate::Message;



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

    let autoscroll_checkbox = checkbox(use_autoscroll)
        .label("autoscroll")
        .on_toggle(Message::SetMonitorAfterUpload)
        .size(16);

    let header = row![back_button, Space::new().width(20), title, ]
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
