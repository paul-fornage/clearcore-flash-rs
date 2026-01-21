mod app;
mod serial;
mod types;
mod ui;

use app::{App, Message};
use iced::{application, window, Size};


fn main() -> iced::Result {
    env_logger::Builder::new().filter_level(log::LevelFilter::Debug)
        .filter(Some("zbus"), log::LevelFilter::Warn)
        .filter(Some("rfd"), log::LevelFilter::Warn)
        .filter(Some("tracing"), log::LevelFilter::Warn)
        .filter(Some("iced_winit"), log::LevelFilter::Warn)
        .filter(Some("wgpu_hal"), log::LevelFilter::Error)
        .filter(Some("iced_wgpu"), log::LevelFilter::Warn)
        .filter(Some("naga"), log::LevelFilter::Warn)
        .filter(Some("cosmic_text"), log::LevelFilter::Warn)
        .filter(Some("wgpu_core"), log::LevelFilter::Warn)
        .filter(Some("sctk"), log::LevelFilter::Warn)
        .filter(Some("winit"), log::LevelFilter::Warn)
        .init();


    log::info!("Starting ClearCore Flasher");

    application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .window(window::Settings {
            size: Size::new(600.0, 500.0),
            resizable: true,
            ..Default::default()
        })
        .run()
}
