use std::sync::OnceLock;
use iced::widget::{button, checkbox, column, container, row, scrollable, text, Space, self};
use iced::{Border, Element, Length, Task, Theme};
use iced_selection::text as selectable_text;
use serialport::SerialPort;
use crate::app::App;
use crate::types::{AppScreen, LogEntry, SerialConfig, Toast};
use crate::Message;

static SCROLLABLE_ID: OnceLock<widget::Id> = OnceLock::new();

#[derive(Debug, Clone)]
pub enum MonitorScreenMessage {
    SerialData(String),
    SerialError(String),
    SetAutoScroll(bool),
}

#[derive(Debug)]
pub struct MonitorState{
    pub serial_config: SerialConfig,
    pub serial_port: Option<Box<dyn SerialPort>>,
    pub logs: Vec<LogEntry>,
    pub auto_scroll: bool,
    pub is_connecting: bool,
    pub is_connected: bool,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            serial_config: SerialConfig::default(),
            serial_port: None,
            logs: Vec::new(),
            auto_scroll: true,
            is_connecting: true,
            is_connected: false,
        }
    }
}


/// Render the monitor screen with serial log view
pub fn monitor_screen(monitor_state: &MonitorState) -> Element<'static, Message> {
    let title = text("Serial Monitor")
        .size(24)
        .style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        });

    let back_button = button(text("← Back").size(16))
        .on_press(Message::BackToMain)
        .padding(8);

    let autoscroll_checkbox = checkbox(monitor_state.auto_scroll)
        .label("autoscroll")
        .on_toggle(|enabled| Message::MonitorScreen(MonitorScreenMessage::SetAutoScroll(enabled)))
        .size(16);

    let header = row![back_button, Space::new().width(20), title, autoscroll_checkbox]
        .spacing(10)
        .align_y(iced::Alignment::Center);

    let log_text = monitor_state.logs
        .iter()
        .map(|entry| format!("[{}] {}", entry.timestamp, entry.message))
        .collect::<Vec<_>>()
        .join("\n");

    let log_view = scrollable(
        container(
            selectable_text(log_text)
                .font(iced::Font::MONOSPACE)
                .size(14)
        ).style(|theme: &Theme| {
            container::Style {
                background: Some(theme.palette().background.into()),
                border: Border{
                    width: 2.0,
                    color: theme.palette().primary.scale_alpha(0.5),
                    radius: iced::border::radius(10.0)
                },
                ..Default::default()
            }
        })
            .padding(10)
            .width(Length::Fill)
    )
        .id(SCROLLABLE_ID.get_or_init(widget::Id::unique).clone())
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


impl App{
    pub fn handle_monitor_screen_message(&mut self, msg: MonitorScreenMessage) -> Task<Message>{
        match self.screen{
            AppScreen::Monitor(ref mut monitor_state) => {
                match msg {
                    MonitorScreenMessage::SerialData(line) => {
                        // Add the line to logs (already contains newline)
                        let trimmed = line.trim_end();
                        if !trimmed.is_empty() {
                            monitor_state.logs.push(LogEntry::new(trimmed.to_string()));
                        }
                    }

                    MonitorScreenMessage::SerialError(err) => {
                        self.toast = Some(Toast::error(format!("Serial error: {}", err)));
                        log::error!("Serial error: {}", err);
                        monitor_state.is_connected = false;
                        // Stay on monitor screen but stop receiving data
                    }

                    MonitorScreenMessage::SetAutoScroll(new_value) => {
                        log::info!("Auto scroll toggled to: {}", new_value);
                        monitor_state.auto_scroll = new_value;
                    }
                }
            }
            _ => {
                log::error!("Received serial data message on non-monitor screen");
            }
        }

        Task::none()
    }
}