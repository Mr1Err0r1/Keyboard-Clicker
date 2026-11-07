use eframe::egui::{Color32, Context, Rounding, Stroke, Visuals};

pub const THEMES: &[(&str, &str, &str)] = &[
    ("dark", "theme_midnight", "#0B1220"),
    ("light", "theme_daylight", "#F2F6FB"),
    ("cyberpunk", "theme_cyberpunk", "#160A25"),
    ("hacker", "theme_terminal", "#07140D"),
    ("modern", "theme_glass", "#171A24"),
    ("synthwave", "theme_synthwave", "#170B2B"),
    ("arctic", "theme_arctic", "#263340"),
    ("kawaii", "theme_kawaii", "#FFF1F7"),
];

pub const BACKGROUNDS: &[(&str, &str)] = &[
    ("solid", "background_solid"),
    ("dots", "background_dots"),
    ("grid", "background_grid"),
    ("blueprint", "background_blueprint"),
    ("scanlines", "background_scanlines"),
    ("hex", "background_hex"),
    ("circuit", "background_circuit"),
    ("aurora", "background_aurora"),
    ("horizon", "background_horizon"),
    ("matrix", "background_matrix"),
    ("stars", "background_stars"),
    ("radar", "background_radar"),
    ("hearts", "background_hearts"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasBackground {
    Solid,
    Dots,
    Grid,
    Blueprint,
    Scanlines,
    Hex,
    Circuit,
    Aurora,
    Horizon,
    Matrix,
    Stars,
    Radar,
    Hearts,
}

impl CanvasBackground {
    pub fn from_id(value: &str) -> Option<Self> {
        Some(match value {
            "solid" | "none" => Self::Solid,
            "dots" => Self::Dots,
            "grid" => Self::Grid,
            "blueprint" => Self::Blueprint,
            "scanlines" => Self::Scanlines,
            "hex" => Self::Hex,
            "circuit" => Self::Circuit,
            "aurora" => Self::Aurora,
            "horizon" => Self::Horizon,
            "matrix" => Self::Matrix,
            "stars" => Self::Stars,
            "radar" => Self::Radar,
            "hearts" => Self::Hearts,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub app_background: Color32,
    pub panel: Color32,
    pub panel_alt: Color32,
    pub canvas: Color32,
    pub border: Color32,
    pub grid: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub move_node: Color32,
    pub keyboard_node: Color32,
    pub wait_node: Color32,
    pub condition_node: Color32,
    pub loop_node: Color32,
    pub variable_node: Color32,
    pub end_node: Color32,
    pub is_light: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct StudioAppearance {
    pub colors: ThemeColors,
    pub background: CanvasBackground,
    pub background_intensity: f32,
    pub glow_strength: f32,
    pub snap_to_grid: bool,
    pub animated_edges: bool,
    pub background_animations: bool,
    pub crt_overlay: bool,
}

pub fn is_supported_theme(value: &str) -> bool {
    THEMES.iter().any(|(id, _, _)| *id == value)
}

pub fn is_supported_background(value: &str) -> bool {
    CanvasBackground::from_id(value).is_some()
}

#[allow(clippy::too_many_arguments)]
pub fn studio_appearance(
    theme: &str,
    accent_hex: &str,
    background: &str,
    background_intensity: u8,
    glow_intensity: u8,
    snap_to_grid: bool,
    animated_edges: bool,
    background_animations: bool,
    crt_overlay: bool,
) -> StudioAppearance {
    StudioAppearance {
        colors: colors_for(theme, accent_hex),
        background: CanvasBackground::from_id(background).unwrap_or(CanvasBackground::Grid),
        background_intensity: f32::from(background_intensity.min(100)) / 100.0,
        glow_strength: f32::from(glow_intensity.min(100)) / 100.0,
        snap_to_grid,
        animated_edges,
        background_animations,
        crt_overlay,
    }
}

pub fn colors_for(theme: &str, accent_hex: &str) -> ThemeColors {
    let accent = parse_color(accent_hex).unwrap_or(Color32::from_rgb(108, 140, 255));
    match theme {
        "kawaii" => ThemeColors {
            app_background: Color32::from_rgb(255, 242, 248),
            panel: Color32::from_rgb(255, 249, 252),
            panel_alt: Color32::from_rgb(252, 230, 241),
            canvas: Color32::from_rgb(255, 248, 252),
            border: Color32::from_rgb(235, 186, 211),
            grid: Color32::from_rgb(238, 208, 225),
            text: Color32::from_rgb(81, 45, 65),
            muted: Color32::from_rgb(146, 104, 128),
            accent,
            success: Color32::from_rgb(62, 173, 126),
            warning: Color32::from_rgb(197, 135, 48),
            danger: Color32::from_rgb(204, 69, 105),
            move_node: Color32::from_rgb(69, 173, 183),
            keyboard_node: Color32::from_rgb(173, 102, 196),
            wait_node: Color32::from_rgb(211, 147, 47),
            condition_node: Color32::from_rgb(146, 108, 204),
            loop_node: Color32::from_rgb(204, 105, 139),
            variable_node: Color32::from_rgb(75, 136, 194),
            end_node: Color32::from_rgb(149, 125, 149),
            is_light: true,
        },
        "light" => ThemeColors {
            app_background: Color32::from_rgb(239, 244, 250),
            panel: Color32::from_rgb(248, 250, 253),
            panel_alt: Color32::from_rgb(231, 237, 245),
            canvas: Color32::from_rgb(244, 247, 251),
            border: Color32::from_rgb(204, 214, 227),
            grid: Color32::from_rgb(210, 220, 233),
            text: Color32::from_rgb(28, 39, 55),
            muted: Color32::from_rgb(99, 113, 132),
            accent,
            success: Color32::from_rgb(22, 148, 102),
            warning: Color32::from_rgb(199, 126, 28),
            danger: Color32::from_rgb(204, 64, 76),
            move_node: Color32::from_rgb(17, 152, 174),
            keyboard_node: Color32::from_rgb(31, 153, 113),
            wait_node: Color32::from_rgb(196, 133, 38),
            condition_node: Color32::from_rgb(128, 87, 201),
            loop_node: Color32::from_rgb(199, 102, 63),
            variable_node: Color32::from_rgb(50, 120, 194),
            end_node: Color32::from_rgb(112, 126, 145),
            is_light: true,
        },
        "cyberpunk" => dark_colors(
            accent,
            Color32::from_rgb(17, 8, 30),
            Color32::from_rgb(29, 15, 46),
            Color32::from_rgb(37, 20, 57),
            Color32::from_rgb(74, 39, 91),
            Color32::from_rgb(42, 27, 66),
            Color32::from_rgb(244, 234, 255),
            Color32::from_rgb(183, 157, 204),
        ),
        "hacker" => dark_colors(
            accent,
            Color32::from_rgb(7, 18, 11),
            Color32::from_rgb(11, 29, 17),
            Color32::from_rgb(15, 38, 22),
            Color32::from_rgb(31, 73, 42),
            Color32::from_rgb(20, 47, 27),
            Color32::from_rgb(220, 255, 226),
            Color32::from_rgb(119, 174, 131),
        ),
        "modern" => dark_colors(
            accent,
            Color32::from_rgb(22, 25, 35),
            Color32::from_rgb(34, 38, 51),
            Color32::from_rgb(42, 47, 62),
            Color32::from_rgb(62, 69, 87),
            Color32::from_rgb(47, 52, 68),
            Color32::from_rgb(246, 245, 244),
            Color32::from_rgb(165, 170, 186),
        ),
        "synthwave" => dark_colors(
            accent,
            Color32::from_rgb(20, 10, 40),
            Color32::from_rgb(34, 16, 59),
            Color32::from_rgb(44, 22, 74),
            Color32::from_rgb(78, 39, 104),
            Color32::from_rgb(54, 27, 84),
            Color32::from_rgb(255, 237, 255),
            Color32::from_rgb(195, 158, 213),
        ),
        "arctic" => dark_colors(
            accent,
            Color32::from_rgb(32, 43, 55),
            Color32::from_rgb(43, 56, 70),
            Color32::from_rgb(51, 66, 81),
            Color32::from_rgb(74, 92, 108),
            Color32::from_rgb(57, 73, 89),
            Color32::from_rgb(237, 245, 251),
            Color32::from_rgb(172, 191, 208),
        ),
        _ => dark_colors(
            accent,
            Color32::from_rgb(11, 18, 32),
            Color32::from_rgb(18, 28, 45),
            Color32::from_rgb(25, 37, 57),
            Color32::from_rgb(44, 59, 82),
            Color32::from_rgb(31, 45, 66),
            Color32::from_rgb(239, 244, 252),
            Color32::from_rgb(153, 166, 186),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn dark_colors(
    accent: Color32,
    app_background: Color32,
    panel: Color32,
    panel_alt: Color32,
    border: Color32,
    grid: Color32,
    text: Color32,
    muted: Color32,
) -> ThemeColors {
    ThemeColors {
        app_background,
        panel,
        panel_alt,
        canvas: app_background,
        border,
        grid,
        text,
        muted,
        accent,
        success: Color32::from_rgb(69, 205, 147),
        warning: Color32::from_rgb(242, 180, 79),
        danger: Color32::from_rgb(236, 97, 113),
        move_node: Color32::from_rgb(55, 196, 211),
        keyboard_node: Color32::from_rgb(76, 206, 147),
        wait_node: Color32::from_rgb(242, 180, 79),
        condition_node: Color32::from_rgb(178, 130, 247),
        loop_node: Color32::from_rgb(242, 137, 93),
        variable_node: Color32::from_rgb(83, 166, 229),
        end_node: Color32::from_rgb(142, 158, 180),
        is_light: false,
    }
}

pub fn apply(ctx: &Context, theme: &str, accent_hex: &str) {
    let palette = colors_for(theme, accent_hex);
    let mut visuals = if palette.is_light {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    visuals.window_fill = palette.panel;
    visuals.panel_fill = palette.app_background;
    visuals.extreme_bg_color = palette.canvas;
    visuals.faint_bg_color = palette.panel_alt;
    visuals.code_bg_color = palette.panel_alt;
    visuals.override_text_color = Some(palette.text);
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.inactive.bg_fill = palette.panel_alt;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.text);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.text);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, palette.border);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.border);
    if theme == "kawaii" {
        visuals.widgets.noninteractive.weak_bg_fill = palette.border;
        visuals.widgets.inactive.weak_bg_fill = palette.panel_alt;
    }
    visuals.widgets.hovered.bg_fill =
        accent_linear(palette.accent, if palette.is_light { 0.18 } else { 0.3 });
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.2_f32, palette.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.text);
    visuals.widgets.active.bg_fill = palette.accent;
    visuals.widgets.active.bg_stroke = Stroke::new(1.2_f32, palette.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, contrast_text(palette.accent));
    visuals.selection.bg_fill = accent_linear(palette.accent, 0.32);
    visuals.selection.stroke = Stroke::new(1.2_f32, palette.accent);
    let rounding = if theme == "kawaii" {
        12.0
    } else if theme == "modern" {
        10.0
    } else if theme == "cyberpunk" {
        2.0
    } else {
        6.0
    };
    visuals.window_rounding = Rounding::same(rounding);
    visuals.menu_rounding = Rounding::same(rounding);
    visuals.widgets.noninteractive.rounding = Rounding::same(rounding.min(6.0));
    visuals.widgets.inactive.rounding = Rounding::same(rounding.min(6.0));
    visuals.widgets.hovered.rounding = Rounding::same(rounding.min(6.0));
    visuals.widgets.active.rounding = Rounding::same(rounding.min(6.0));
    ctx.set_visuals(visuals);
}

fn accent_linear(color: Color32, amount: f32) -> Color32 {
    let alpha = (amount.clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn contrast_text(color: Color32) -> Color32 {
    let luminance = 0.2126 * f32::from(color.r())
        + 0.7152 * f32::from(color.g())
        + 0.0722 * f32::from(color.b());
    if luminance > 145.0 {
        Color32::from_rgb(16, 22, 33)
    } else {
        Color32::WHITE
    }
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

pub fn format_color(color: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
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

    #[test]
    fn formats_picker_colors_as_six_digit_hex() {
        let color = Color32::from_rgb(0x12, 0xAB, 0xF0);
        assert_eq!(format_color(color), "#12ABF0");
        assert_eq!(parse_color(&format_color(color)), Some(color));
    }

    #[test]
    fn registers_all_reference_themes_and_canvas_backgrounds() {
        assert_eq!(THEMES.len(), 8);
        assert_eq!(BACKGROUNDS.len(), 13);
        assert!(THEMES.iter().all(|(id, _, _)| is_supported_theme(id)));
        assert!(BACKGROUNDS
            .iter()
            .all(|(id, _)| is_supported_background(id)));
        assert!(!is_supported_theme("neon"));
        assert!(!is_supported_background("unknown"));
        assert!(is_supported_background("none"));
    }

    #[test]
    fn theme_palette_uses_custom_accent_without_losing_semantic_colors() {
        let palette = colors_for("cyberpunk", "#12ABF0");
        assert_eq!(palette.accent, Color32::from_rgb(0x12, 0xAB, 0xF0));
        assert_eq!(palette.wait_node, Color32::from_rgb(242, 180, 79));
        assert!(!palette.is_light);
        assert!(colors_for("light", "#12ABF0").is_light);
        assert!(colors_for("kawaii", "#12ABF0").is_light);
    }

    #[test]
    fn studio_appearance_preserves_animation_preference_and_new_backgrounds() {
        let appearance = studio_appearance(
            "kawaii", "#12ABF0", "hearts", 70, 35, true, true, false, false,
        );
        assert_eq!(appearance.background, CanvasBackground::Hearts);
        assert!(!appearance.background_animations);
        assert!(appearance.colors.is_light);
    }
}
