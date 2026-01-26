use cansi::v3::CategorisedSlice;
use iced::{Color, Font};
use iced::font::{Style, Weight};
use iced::widget::text;
// TODO: [cansi](https://lib.rs/crates/cansi)
use iced_selection::span as selectable_span;

const ESC: &str = "\x1b[";
const RESET: &str = "\x1b[0m";


pub fn extract_wrapped_ansi_fg_color(line: &str) -> Option<(Color, &str)> {
    extract_wrapped_ansi_fg_u8(line).and_then(|(code, content)| ansi_fg_u8_to_color(code).map(|fg| (fg, content)))
}



/// If `line` looks like: `ESC[` <u8> `m` <content> `ESC[0m`
/// returns (code, content_without_wrapping).
///
/// For now: only supports a single numeric code (no `;` parsing).
/// Later: you can extend the parsing to stop at `;` and/or parse multiple codes.
pub fn extract_wrapped_ansi_fg_u8(line: &str) -> Option<(u8, &str)> {
    if !line.starts_with(ESC) || !line.ends_with(RESET) {
        return None;
    }

    // Find the terminating 'm' for the opening SGR
    let after_esc = &line[ESC.len()..];
    let m_pos = after_esc.find('m')?;
    let inside = &after_esc[..m_pos];

    // For now, explicitly reject `;`-style sequences to keep it simple.
    if inside.contains(';') {
        return None;
    }

    let code: u8 = inside.parse().ok()?;

    let content_start = ESC.len() + m_pos + 1;
    let content_end = line.len() - RESET.len();
    if content_start > content_end {
        return None;
    }

    let content = &line[content_start..content_end];
    Some((code, content))
}

/// Compile-time mapping for ANSI standard + bright foreground colors.
///
/// Standard foreground: 30..=37
/// Bright foreground:   90..=97  (treated as (code - 60) then brightened)
///
/// Returns `None` for codes you don't handle (including reset 0).
pub fn ansi_fg_u8_to_color(code: u8) -> Option<Color> {
    // Reset / "clear" shouldn't apply a foreground color.
    if code == 0 {
        return None;
    }

    if (30..=37).contains(&code) {
        return Some(ansi_standard_fg_to_color(code));
    }

    // Bright colors are typically 60 higher (90..=97 vs 30..=37).
    if (90..=97).contains(&code) {
        let base = ansi_standard_fg_to_color(code - 60);
        return Some(brighten(base));
    }

    None
}

fn ansi_standard_fg_to_color(code: u8) -> Color {
    match code {
        30 => Color::from_rgb(0.0, 0.0, 0.0),       // black
        31 => Color::from_rgb(0.80, 0.0, 0.0),      // red
        32 => Color::from_rgb(0.0, 0.60, 0.0),      // green
        33 => Color::from_rgb(0.80, 0.60, 0.0),     // yellow
        34 => Color::from_rgb(0.10, 0.30, 0.90),    // blue
        35 => Color::from_rgb(0.70, 0.20, 0.70),    // magenta
        36 => Color::from_rgb(0.0, 0.70, 0.70),     // cyan
        37 => Color::from_rgb(0.85, 0.85, 0.85),    // white/gray
        _ => Color::from_rgb(0.85, 0.85, 0.85),     // unreachable for callers, but safe
    }
}

/// Simple "bright" transform: move each channel toward 1.0.
/// Tweak factor to taste.
fn brighten(c: Color) -> Color {
    let factor = 0.35; // 0.0 = no change, 1.0 = full white
    Color::from_rgb(
        c.r + (1.0 - c.r) * factor,
        c.g + (1.0 - c.g) * factor,
        c.b + (1.0 - c.b) * factor,
    )
}

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
        .font(ansi_font(Font::MONOSPACE, &source))
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
