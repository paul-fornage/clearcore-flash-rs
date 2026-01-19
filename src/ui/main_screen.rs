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
            .on_toggle(Message::SetMonitorAfterUpload)
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