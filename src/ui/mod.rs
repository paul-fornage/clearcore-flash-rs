pub mod main_screen;
pub mod monitor_screen;
pub mod upload_screen;
pub mod toast;
mod common;
pub mod download_screen;
mod equal_height_row;
mod ansi_color;
use iced::Font;

pub const JETBRAINS_MONO: Font = Font::with_name("JetBrains Mono");

pub use toast::with_toast;
pub use main_screen::{main_screen, MainScreenMessage};
pub use monitor_screen::{monitor_screen, MonitorScreenMessage};
pub use upload_screen::{upload_screen, UploadScreenMessage};
pub use download_screen::{download_screen, DownloadScreenMessage};
