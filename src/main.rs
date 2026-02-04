// makes windows not launch the EXE with a terminal
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]


mod app;
mod serial;
mod types;
mod ui;

use app::{App, Message};
use iced::{application, window, Size};
use ui::JETBRAINS_MONO;



fn main() -> iced::Result {
    env_logger::Builder::new().filter_level(log::LevelFilter::Trace)
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
        .filter(Some("iced_graphics"), log::LevelFilter::Debug)
        .filter(Some("calloop"), log::LevelFilter::Debug)
        .init();


    log::info!("Starting ClearCore Flasher");

    application(App::new, App::update, App::view)
        .font(include_bytes!("../assets/JetBrainsMono[wght].ttf"))
        .font(include_bytes!("../assets/JetBrainsMono-Italic[wght].ttf"))
        .default_font(JETBRAINS_MONO)
        .subscription(App::subscription)
        .theme(App::theme)
        .title("ClearCore Flasher")
        .window(window::Settings {
            size: Size::new(800.0, 600.0),
            resizable: true,
            ..Default::default()
        })
        .run()
}
