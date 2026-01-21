use std::fmt::Display;
use iced::widget::{button, checkbox, column, container, row, scrollable, text, Space, self, operation};
use iced::{clipboard, Background, Border, Color, Element, Length, Task, Theme};
use iced::border::Radius;
use iced::widget::scrollable::RelativeOffset;
use iced_selection::text as selectable_text;


use crate::app::App;
use crate::types::{AppScreen, LogEntry, SerialConfig};
use crate::Message;
use crate::ui::toast::Toast;

const CONST_SCROLLABLE_ID: widget::Id = widget::Id::new("serial monitor scrollable widget id");

#[derive(Debug, Clone)]
pub enum MonitorScreenMessage {
    ConnectionStateChanged(MonitorConnectionState),
    SerialData(String),
    JumpToBottom,
    CopyLogs,
    SaveLogs,
    SaveLogsFinished(Result<bool, String>),
}

#[derive(Debug)]
pub struct MonitorState{
    pub serial_config: SerialConfig,
    pub connection_state: MonitorConnectionState,
    pub logs: Vec<LogEntry>,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            serial_config: SerialConfig::default(),
            connection_state: MonitorConnectionState::Disconnected, // Starts disconnected until entered
            logs: Vec::new(),
        }
    }
}



/// Represents the current state of the serial connection
#[derive(Debug, Clone, PartialEq)]
pub enum MonitorConnectionState {
    Disconnected,
    Searching,
    Connecting(String),
    Connected(String),
    Error(String),
}

impl Default for MonitorConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}



// Helper for the banner
fn connection_banner(state: &MonitorConnectionState) -> Element<'static, Message> {
    let (bg_color, content) = match state {
        MonitorConnectionState::Disconnected => (
            Color::from_rgb(0.5, 0.5, 0.5),
            text("Disconnected")
        ),
        MonitorConnectionState::Searching => (
            Color::from_rgb(0.8, 0.8, 0.0), // Yellow-ish
            text("Searching for ClearCore...")
        ),
        MonitorConnectionState::Connecting(port) => (
            Color::from_rgb(0.0, 0.5, 0.8), // Blue-ish
            text(format!("Connecting to {}...", port))
        ),
        MonitorConnectionState::Connected(port) => (
            Color::from_rgb(0.0, 0.8, 0.0), // Green
            text(format!("Connected to {}", port))
        ),
        MonitorConnectionState::Error(err) => (
            Color::from_rgb(0.9, 0.1, 0.1), // Red
            text(format!("Error: {}", err))
        ),
    };

    container(content.size(14).style(|_| text::Style{ color: Some(Color::WHITE) }))
        .width(Length::Fill)
        .padding(5)
        .align_x(iced::Alignment::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(bg_color)),
            border: Border{
                width: 3.0,
                color: bg_color.scale_alpha(0.5),
                radius: Radius::new(5.0),
            },
            ..Default::default()
        })
        .into()
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

    let header = row![back_button, Space::new().width(20), title]
        .spacing(10)
        .align_y(iced::Alignment::Center);

    // Banner sits below header, above logs
    let banner = connection_banner(&monitor_state.connection_state);

    let log_text = monitor_state.logs
        .iter()
        .map(|entry| format!("{entry}"))
        .collect::<Vec<_>>()
        .join("\n");


    let log_view = scrollable(
        selectable_text(log_text)
            .font(iced::Font::MONOSPACE)
            .size(14)
    )
        .id(CONST_SCROLLABLE_ID.clone())
        .height(Length::Fill)
        .width(Length::Fill);


    let is_connected = matches!(monitor_state.connection_state, MonitorConnectionState::Connected(_));

    let log_view_container = container(log_view).style(move |theme: &Theme| {
        let border_color = if is_connected {
            theme.palette().primary.scale_alpha(0.5)
        } else {
            Color::from_rgb(0.8, 0.0, 0.0) // Red warning border
        };

        container::Style {
            background: Some(theme.palette().background.into()),
            border: Border{
                width: 3.0,
                color: border_color,
                radius: Radius::new(10.0)
            },
            ..Default::default()
        }
    }).padding(10);


    let jump_btn = button("Jump to bottom")
        .on_press(Message::MonitorScreen(MonitorScreenMessage::JumpToBottom))
        .padding(5);

    let copy_btn = button("Copy All")
        .on_press(Message::MonitorScreen(MonitorScreenMessage::CopyLogs))
        .padding(5);

    let save_btn = button("Save to File")
        .on_press(Message::MonitorScreen(MonitorScreenMessage::SaveLogs))
        .padding(5);

    let bottom_controls = row![jump_btn, copy_btn, save_btn]
        .spacing(20)
        .align_y(iced::Alignment::Center);

    let content = column![header, banner, log_view_container, bottom_controls]
        .spacing(10)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}


impl App{
    pub fn handle_monitor_screen_message(&mut self, msg: MonitorScreenMessage) -> Task<Message> {
        match self.screen {
            AppScreen::Monitor(ref mut monitor_state) => {
                match msg {
                    MonitorScreenMessage::SerialData(line) => {
                        let trimmed = line.trim_end();
                        if !trimmed.is_empty() {
                            monitor_state.logs.push(LogEntry::new_now(trimmed.to_string()));
                        }
                    }
                    MonitorScreenMessage::ConnectionStateChanged(new_state) => {
                        monitor_state.connection_state = new_state;
                    }
                    MonitorScreenMessage::JumpToBottom => {
                        return operation::snap_to(CONST_SCROLLABLE_ID, RelativeOffset::END);
                    }
                    MonitorScreenMessage::CopyLogs => {
                        let content = monitor_state.logs
                            .iter()
                            .map(|entry| format!("{entry}"))
                            .collect::<Vec<_>>()
                            .join("\n");

                        // Iced has no way to verify the result, I assume it's because it doesn't fail
                        self.toast = Some(Toast::info("Logs copied to clipboard"));
                        return clipboard::write(content);
                    }
                    MonitorScreenMessage::SaveLogs => {
                        let content = monitor_state.logs
                            .iter()
                            .map(|entry| format!("{entry}"))
                            .collect::<Vec<_>>()
                            .join("\n");

                        return Task::future(async move {
                            let file = rfd::AsyncFileDialog::new()
                                .set_title("Save Serial Log")
                                .set_file_name("serial_log.txt")
                                .save_file()
                                .await;

                            if let Some(handle) = file {
                                match handle.write(content.as_bytes()).await {
                                    Ok(_) => {
                                        Message::MonitorScreen(MonitorScreenMessage::SaveLogsFinished(Ok(true)))
                                    },
                                    Err(e) => {
                                        log::error!("Failed to save file: {}", e);
                                        Message::MonitorScreen(MonitorScreenMessage::SaveLogsFinished(Err(format!("Failed to save file: {}", e))))
                                    }
                                }
                            } else {
                                Message::MonitorScreen(MonitorScreenMessage::SaveLogsFinished(Ok(false)))
                            }
                        });
                    }
                    MonitorScreenMessage::SaveLogsFinished(Ok(true)) => {
                        self.toast = Some(Toast::info("Serial log saved successfully"));
                    }
                    MonitorScreenMessage::SaveLogsFinished(Ok(false)) => {
                        // user cancelled?
                    }
                    MonitorScreenMessage::SaveLogsFinished(Err(err)) => {
                        self.toast = Some(Toast::error(format!("Failed to save serial log: {}", err)));
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