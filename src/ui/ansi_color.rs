use cansi::v3::CategorisedSlice;
use iced::{Font};
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

    selectable_span(source.text.to_string())
        .color_maybe(source.fg.map(cansi_color_to_iced_color))
        .background_maybe(source.bg.map(cansi_color_to_iced_color))
        .font(ansi_font(JETBRAINS_MONO, &source))
        .strikethrough(source.strikethrough.unwrap_or(false))
        .underline(source.underline.unwrap_or(false))

}

pub fn cansi_color_to_iced_color(color: cansi::v3::Color) -> iced::Color {
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

    iced::Color::from_rgb8(r, g, b)
}


#[cfg(test)]
mod tests {
    use super::*;
    use cansi::v3::categorise_text;
    use iced::font::{Style, Weight};

    const RESET: &str = "\x1b[0m";
    const GREEN: &str = "\x1b[32m";
    const CYAN: &str = "\x1b[36m";
    const MAGENTA: &str = "\x1b[35m";
    const BOLD: &str = "\x1b[1m";
    const DIM: &str = "\x1b[2m";
    const ITALIC: &str = "\x1b[3m";
    const UNDERLINE: &str = "\x1b[4m";

    fn spans_from(input: &str) -> Vec<iced::widget::text::Span<'static, (), iced::Font>> {
        categorise_text(input).into_iter().map(ansi_color_to_span).collect()
    }

    fn iced_green() -> iced::Color { cansi_color_to_iced_color(cansi::v3::Color::Green) }
    fn iced_cyan() -> iced::Color { cansi_color_to_iced_color(cansi::v3::Color::Cyan) }
    fn iced_magenta() -> iced::Color { cansi_color_to_iced_color(cansi::v3::Color::Magenta) }

    /// ANSI_CYAN, ANSI_BOLD, "[echo]", ANSI_RESET, " ",
    /// ANSI_UNDERLINE, ANSI_GREEN, "<<<", ANSI_ITALIC, command,
    /// ANSI_UNDERLINE, ">>>", ANSI_RESET
    #[test]
    fn test_echo_command() {
        let command = "mycommand";
        let input = format!("{CYAN}{BOLD}[echo]{RESET} {UNDERLINE}{GREEN}<<<{ITALIC}{command}{UNDERLINE}>>>{RESET}");
        let spans = spans_from(&input);

        assert_eq!(spans.len(), 5);

        // "[echo]": cyan, bold
        assert_eq!(spans[0].text.as_ref(), "[echo]");
        assert_eq!(spans[0].color, Some(iced_cyan()));
        assert_eq!(spans[0].font.unwrap().weight, Weight::Bold);
        assert_eq!(spans[0].font.unwrap().style, Style::Normal);
        assert!(!spans[0].underline);

        // " ": reset — no color, normal
        assert_eq!(spans[1].text.as_ref(), " ");
        assert_eq!(spans[1].color, None);
        assert_eq!(spans[1].font.unwrap().weight, Weight::Normal);
        assert_eq!(spans[1].font.unwrap().style, Style::Normal);
        assert!(!spans[1].underline);

        // "<<<": green, underline
        assert_eq!(spans[2].text.as_ref(), "<<<");
        assert_eq!(spans[2].color, Some(iced_green()));
        assert_eq!(spans[2].font.unwrap().weight, Weight::Normal);
        assert_eq!(spans[2].font.unwrap().style, Style::Normal);
        assert!(spans[2].underline);

        // command: green, underline, italic
        assert_eq!(spans[3].text.as_ref(), command);
        assert_eq!(spans[3].color, Some(iced_green()));
        assert_eq!(spans[3].font.unwrap().style, Style::Italic);
        assert!(spans[3].underline);

        // ">>>": green, underline, italic (UNDERLINE repeated = same state)
        assert_eq!(spans[4].text.as_ref(), ">>>");
        assert_eq!(spans[4].color, Some(iced_green()));
        assert_eq!(spans[4].font.unwrap().style, Style::Italic);
        assert!(spans[4].underline);
    }

    /// ANSI_GREEN, ANSI_BOLD, "[heartbeat]", ANSI_RESET,
    /// ANSI_DIM, " alive t_ms=", ANSI_RESET, Milliseconds()
    #[test]
    fn test_heartbeat() {
        let ms = "9876";
        let input = format!("{GREEN}{BOLD}[heartbeat]{RESET}{DIM} alive t_ms={RESET}{ms}");
        let spans = spans_from(&input);

        assert_eq!(spans.len(), 3);

        // "[heartbeat]": green, bold
        assert_eq!(spans[0].text.as_ref(), "[heartbeat]");
        assert_eq!(spans[0].color, Some(iced_green()));
        assert_eq!(spans[0].font.unwrap().weight, Weight::Bold);
        assert_eq!(spans[0].font.unwrap().style, Style::Normal);

        // " alive t_ms=": dim only (reset cleared color)
        assert_eq!(spans[1].text.as_ref(), " alive t_ms=");
        assert_eq!(spans[1].color, None);
        assert_eq!(spans[1].font.unwrap().weight, Weight::Light);
        assert_eq!(spans[1].font.unwrap().style, Style::Normal);

        // milliseconds: all default (second reset)
        assert_eq!(spans[2].text.as_ref(), ms);
        assert_eq!(spans[2].color, None);
        assert_eq!(spans[2].font.unwrap().weight, Weight::Normal);
        assert_eq!(spans[2].font.unwrap().style, Style::Normal);
    }

    /// ANSI_MAGENTA, "[boot]", ANSI_RESET, " usb echo firmware ready"
    #[test]
    fn test_boot_message() {
        let input = format!("{MAGENTA}[boot]{RESET} usb echo firmware ready");
        let spans = spans_from(&input);

        assert_eq!(spans.len(), 2);

        // "[boot]": magenta
        assert_eq!(spans[0].text.as_ref(), "[boot]");
        assert_eq!(spans[0].color, Some(iced_magenta()));
        assert_eq!(spans[0].font.unwrap().weight, Weight::Normal);
        assert_eq!(spans[0].font.unwrap().style, Style::Normal);
        assert!(!spans[0].underline);

        // " usb echo firmware ready": no attrs
        assert_eq!(spans[1].text.as_ref(), " usb echo firmware ready");
        assert_eq!(spans[1].color, None);
        assert_eq!(spans[1].font.unwrap().weight, Weight::Normal);
        assert!(!spans[1].underline);
    }

    /// ANSI_BOLD, "[format]", ANSI_RESET, " ",
    /// ANSI_BOLD, "bold ", ANSI_DIM, "dim ",
    /// ANSI_ITALIC, "italic ", ANSI_UNDERLINE, "underline ", ANSI_RESET
    #[test]
    fn test_format_styles() {
        let input = format!("{BOLD}[format]{RESET} {BOLD}bold {DIM}dim {ITALIC}italic {UNDERLINE}underline {RESET}");
        let spans = spans_from(&input);

        assert_eq!(spans.len(), 6);

        // "[format]": bold
        assert_eq!(spans[0].text.as_ref(), "[format]");
        assert_eq!(spans[0].font.unwrap().weight, Weight::Bold);
        assert_eq!(spans[0].font.unwrap().style, Style::Normal);
        assert!(!spans[0].underline);

        // " ": reset — normal
        assert_eq!(spans[1].text.as_ref(), " ");
        assert_eq!(spans[1].font.unwrap().weight, Weight::Normal);
        assert_eq!(spans[1].font.unwrap().style, Style::Normal);

        // "bold ": bold
        assert_eq!(spans[2].text.as_ref(), "bold ");
        assert_eq!(spans[2].font.unwrap().weight, Weight::Bold);
        assert_eq!(spans[2].font.unwrap().style, Style::Normal);
        assert!(!spans[2].underline);

        // "dim ": faint (DIM replaces BOLD in intensity)
        assert_eq!(spans[3].text.as_ref(), "dim ");
        assert_eq!(spans[3].font.unwrap().weight, Weight::Light);
        assert_eq!(spans[3].font.unwrap().style, Style::Normal);
        assert!(!spans[3].underline);

        // "italic ": faint + italic
        assert_eq!(spans[4].text.as_ref(), "italic ");
        assert_eq!(spans[4].font.unwrap().weight, Weight::Light); // FAILS
        assert_eq!(spans[4].font.unwrap().style, Style::Italic);
        assert!(!spans[4].underline);

        // "underline ": faint + italic + underline
        assert_eq!(spans[5].text.as_ref(), "underline ");
        assert_eq!(spans[5].font.unwrap().weight, Weight::Light); // FAILS
        assert_eq!(spans[5].font.unwrap().style, Style::Italic); // FAILS
        assert!(spans[5].underline);
    }
}
