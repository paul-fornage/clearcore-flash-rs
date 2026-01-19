mod app;
mod serial;
mod types;
mod ui;

use app::{App, Message};
use iced::{application, window, Size};

fn main() -> iced::Result {
    env_logger::init();

    log::info!("Starting ClearCore Flasher");

    application(App::new, App::update, App::view)
        .theme(App::theme)
        .window(window::Settings {
            size: Size::new(600.0, 500.0),
            resizable: true,
            ..Default::default()
        })
        .run()
}
