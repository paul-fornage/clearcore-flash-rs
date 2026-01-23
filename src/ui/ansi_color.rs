use iced::Color;

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