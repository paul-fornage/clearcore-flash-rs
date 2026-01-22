use iced::{widget, Border, Color, Length, Renderer, Theme};
use iced::border::Radius;
use iced::widget::{container, scrollable, Container};
use crate::types::LogEntry;
use iced_selection::text as selectable_text;
use crate::app::Message;

pub fn logs_to_container<'a>(logs: &Vec<LogEntry>, id: &widget::Id, color_override: Option<Color>) -> Container<'a, Message, Theme, Renderer> {
    let log_text = logs
        .iter()
        .map(|entry| format!("{entry}"))
        .collect::<Vec<_>>()
        .join("\n");


    let log_view = scrollable(
        selectable_text(log_text)
            .font(iced::Font::MONOSPACE)
            .size(14)
    )
        .id(id.clone())
        .height(Length::Fill)
        .width(Length::Fill);


    container(log_view).style(move |theme: &Theme| {
        let border_color = color_override.unwrap_or(theme.palette().primary.scale_alpha(0.5));

        container::Style {
            background: Some(theme.palette().background.into()),
            border: Border{
                width: 3.0,
                color: border_color,
                radius: Radius::new(10.0)
            },
            ..Default::default()
        }
    }).padding(10)
}


pub enum InformationImportance {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}
impl InformationImportance {
    pub fn to_color(&self) -> Color {
        match self {
            InformationImportance::Trace => Color::from_rgb(0.5, 0.5, 0.5),
            InformationImportance::Debug => Color::from_rgb(0.0, 0.5, 0.5),
            InformationImportance::Info => Color::from_rgb(0.0, 0.5, 0.0),
            InformationImportance::Warning => Color::from_rgb(0.5, 0.5, 0.0),
            InformationImportance::Error => Color::from_rgb(0.8, 0.0, 0.0),
        }
    }
}