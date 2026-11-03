use eframe::egui::{Color32, Context, Rounding, Stroke, Visuals};

pub fn apply(ctx: &Context, theme: &str, accent_hex: &str) {
    let accent = parse_color(accent_hex).unwrap_or(Color32::from_rgb(108, 140, 255));
    let mut visuals = if theme == "light" {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    visuals.selection.bg_fill = accent.linear_multiply(0.34);
    visuals.selection.stroke = Stroke::new(1.2_f32, accent);
    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.bg_fill = accent.linear_multiply(0.55);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.inactive.rounding = Rounding::same(5.0);
    visuals.widgets.hovered.rounding = Rounding::same(5.0);
    visuals.widgets.active.rounding = Rounding::same(5.0);
    ctx.set_visuals(visuals);
}

pub fn parse_color(value: &str) -> Option<Color32> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 || !value.is_ascii() {
        return None;
    }
    let red = u8::from_str_radix(&value[0..2], 16).ok()?;
    let green = u8::from_str_radix(&value[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some(Color32::from_rgb(red, green, blue))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_color_safely() {
        assert_eq!(
            parse_color("#123456"),
            Some(Color32::from_rgb(0x12, 0x34, 0x56))
        );
        assert!(parse_color("bad").is_none());
        assert!(parse_color("😀ab").is_none());
    }
}
