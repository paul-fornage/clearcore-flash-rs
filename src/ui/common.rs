use cansi::categorise_text;
use iced::{widget, Border, Color, Font, Length, Renderer, Theme};
use crate::ui::JETBRAINS_MONO;
use iced::border::Radius;
use iced::widget::{container, scrollable, column, Container, text, Space, row, progress_bar};
use crate::types::{LogEntry, LogMsgType};
use iced_selection::rich_text as selectable_rich_text;
use iced_selection::span as selectable_span;
use iced_selection::text::Span as SelectableSpan;
use crate::app::Message;
use crate::ui::ansi_color::{ansi_color_to_span};

pub fn card<'a>(
    content: impl Into<iced::Element<'a, Message, Theme, Renderer>>,
) -> Container<'a, Message, Theme, Renderer> {
    container(content)
        .style(|theme: &Theme| {
            let bg = theme.palette().background.scale_alpha(0.95);
            let fg = theme.palette().primary.scale_alpha(0.05);
            let mixed_bg = mix_colors(&bg, &fg);
            container::Style {
                background: Some(mixed_bg.into()),
                border: Border {
                    width: 0.0,
                    color: mixed_bg,
                    radius: Radius::new(10.0),
                },
                ..Default::default()
            }
        })
        .padding(16)
}

pub fn logs_to_container(
    logs: &Vec<LogEntry>,
    id: &widget::Id,
    color_override: Option<Color>,
) -> Container<'static, Message, Theme, Renderer> {
    let spans = logs
        .iter()
        .flat_map(|entry| {
            entry.as_spans()
        }).collect::<Vec<SelectableSpan<>>>();

    let log_view = scrollable(
        selectable_rich_text(spans)
            .font(JETBRAINS_MONO)
            .size(14),
    )
        .id(id.clone())
        .height(Length::Fill)
        .width(Length::Fill);

    container(log_view)
        .style(move |theme: &Theme| {
            let border_color = color_override.unwrap_or(theme.palette().primary.scale_alpha(0.5));

            container::Style {
                background: Some(theme.palette().background.into()),
                border: Border {
                    width: 3.0,
                    color: border_color,
                    radius: Radius::new(10.0),
                },
                ..Default::default()
            }
        })
        .padding(10)
}


impl LogEntry {
    pub fn as_spans(&self) -> Vec<SelectableSpan<'static>> {
        let timestamp_span = selectable_span(format!("[{}] ", self.format_timestamp()));

        let preamble = self.message.log_type.as_preamble();
        match preamble {
            Some(preamble) => {
                let preamble_text = preamble.text;
                let preamble_color = preamble.color;
                let preamble_span = selectable_span(preamble_text).color(preamble_color);
                let content_span = selectable_span(format!(": {}\n", self.message.message.trim()));
                vec![timestamp_span, preamble_span, content_span]
            }
            None => {
                let mut content = self.message.message.trim().to_string();
                content.push('\n');
                let result = categorise_text(&content);

                let mut spans: Vec<text::Span<(), Font>> = Vec::with_capacity(result.len()+1);
                spans.push(timestamp_span);
                let mut content_spans = result.into_iter()
                    .map(|c| ansi_color_to_span(c)).collect::<Vec<_>>();
                spans.append(&mut content_spans);

                spans
            }
        }
    }
}


pub struct Preamble {
    pub text: &'static str,
    pub color: Color,
}
impl Preamble {
    pub fn new(text: &'static str, color: Color) -> Self { Self { text, color } }
}

impl LogMsgType{
    
    pub fn as_preamble(&self) -> Option<Preamble> {
        match self{
            LogMsgType::BossaNative => None,
            LogMsgType::ClearCore => None,
            LogMsgType::Trace => Some(Preamble::new("TRACE", Color::from_rgb(0.5, 0.5, 0.5))),
            LogMsgType::Debug => Some(Preamble::new("DEBUG", Color::from_rgb(0.0, 0.5, 0.5))),
            LogMsgType::Info => Some(Preamble::new("INFO", Color::from_rgb(0.0, 0.5, 0.0))),
            LogMsgType::Warn => Some(Preamble::new("WARN", Color::from_rgb(0.5, 0.5, 0.0))),
            LogMsgType::Error => Some(Preamble::new("ERROR", Color::from_rgb(0.8, 0.0, 0.0))),
        }
    }
}


pub fn prog_bar(total: u32, current: u32, name: &str) -> Container<'static, Message, Theme, Renderer> {
    let range = 0f32..=total as f32;
    let current_progress = current as f32;
    let percent = current_progress / total as f32 * 100.0;
    container(column![
        text(name.to_string()).size(24),
        Space::new().height(10),
        row![
            progress_bar(range, current_progress),
            text(format!("{percent:>6.2}% ({}/{} pages)", current, total))
                .size(16).font(JETBRAINS_MONO)
        ]
    ])
}


/// adds wighted by alpha.
pub fn mix_colors(color1: &Color, color2: &Color) -> Color {
    let r = ((color1.r * color1.a) + (color2.r * color2.a)).clamp(0.0, 1.0);
    let g = ((color1.g * color1.a) + (color2.g * color2.a)).clamp(0.0, 1.0);
    let b = ((color1.b * color1.a) + (color2.b * color2.a)).clamp(0.0, 1.0);
    let a = (color1.a + color2.a).clamp(0.0, 1.0);

    Color::from_rgba(r, g, b, a)
}