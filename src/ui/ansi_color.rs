use cansi::v3::CategorisedSlice;
use iced::{Color, Font};
use crate::ui::JETBRAINS_MONO;
use iced::font::{Style, Weight};
use iced::widget::text;

use iced_selection::span as selectable_span;





fn ansi_font(base: Font, source: &CategorisedSlice) -> Font {
    let mut font = base;

    if let Some(intensity) = source.intensity {
        font.weight = match intensity {
            cansi::v3::Intensity::Bold => Weight::Bold,
            cansi::v3::Intensity::Faint => Weight::Light,
            cansi::v3::Intensity::Normal => Weight::Normal,
        };
    }

    if source.italic.unwrap_or(false) {
        font.style = Style::Italic;
    }

    font
}


pub fn ansi_color_to_span(source: CategorisedSlice) -> text::Span<'static, (), Font> {

    /*
        /// The foreground (or text) colour.
        pub fg: Option<Color>,
        /// The background colour.
        pub bg: Option<Color>,

        /// The emphasis state (bold, faint, normal).
        pub intensity: Option<Intensity>,

        /// Italicised.
        pub italic: Option<bool>,
        /// Underlined.
        pub underline: Option<bool>,

        /// Slow blink text.
        pub blink: Option<bool>,
        /// Inverted colours. See [https://en.wikipedia.org/wiki/Reverse_video](https://en.wikipedia.org/wiki/Reverse_video).
        pub reversed: Option<bool>,
        /// Invisible text.
        pub hidden: Option<bool>,
        /// Struck-through.
        pub strikethrough: Option<bool>,
     */
    selectable_span(source.text.to_string())
        .color_maybe(source.fg.map(cansi_color_to_iced_color))
        .background_maybe(source.bg.map(cansi_color_to_iced_color))
        .font(ansi_font(JETBRAINS_MONO, &source))
        .strikethrough(source.strikethrough.unwrap_or(false))
        .underline(source.underline.unwrap_or(false))

}

pub fn cansi_color_to_iced_color(color: cansi::v3::Color) -> Color {
    match color {
        cansi::v3::Color::Black => str_color_to_iced_color("#000000"),
        cansi::v3::Color::Red => str_color_to_iced_color("#a03030"),
        cansi::v3::Color::Green => str_color_to_iced_color("#00a000"),
        cansi::v3::Color::Yellow => str_color_to_iced_color("#a09030"),
        cansi::v3::Color::Blue => str_color_to_iced_color("#2050b0"),
        cansi::v3::Color::Magenta => str_color_to_iced_color("#903090"),
        cansi::v3::Color::Cyan => str_color_to_iced_color("#309090"),
        cansi::v3::Color::White => str_color_to_iced_color("#a0a0a0"),

        cansi::v3::Color::BrightBlack => str_color_to_iced_color("#505050"),
        cansi::v3::Color::BrightRed => str_color_to_iced_color("#f03030"),
        cansi::v3::Color::BrightGreen => str_color_to_iced_color("#00f000"),
        cansi::v3::Color::BrightYellow => str_color_to_iced_color("#f0c030"),
        cansi::v3::Color::BrightBlue => str_color_to_iced_color("#2050f0"),
        cansi::v3::Color::BrightMagenta => str_color_to_iced_color("#f030f0"),
        cansi::v3::Color::BrightCyan => str_color_to_iced_color("#30f0f0"),
        cansi::v3::Color::BrightWhite => str_color_to_iced_color("#f0f0f0"),
    }
}



pub const fn str_color_to_iced_color(s: &str) -> iced::Color {

    const fn hex_nibble(b: u8) -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => 10 + (b - b'a'),
            b'A'..=b'F' => 10 + (b - b'A'),
            _ => panic!("Invalid hex digit in color; expected 0-9, a-f, A-F"),
        }
    }

    const fn hex_byte(hi: u8, lo: u8) -> u8 {
        (hex_nibble(hi) << 4) | hex_nibble(lo)
    }

    let bytes = s.as_bytes();

    if bytes.len() != 7 {
        panic!("Invalid color string format; expected \"#RRGGBB\"");
    }
    if bytes[0] != b'#' {
        panic!("Invalid color string format; expected leading '#'");
    }

    let r = hex_byte(bytes[1], bytes[2]);
    let g = hex_byte(bytes[3], bytes[4]);
    let b = hex_byte(bytes[5], bytes[6]);

    // If this is const in your iced version, you can use it:
    // iced::Color::from_rgb8(r, g, b)

    // Otherwise, build it directly (iced::Color is { r, g, b, a } as f32):
    iced::Color::from_rgb8(r, g, b)
}
