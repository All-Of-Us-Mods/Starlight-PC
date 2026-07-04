use gpui::*;

use crate::backend::services::core_service::{AccentColor, AppTint};

#[derive(Clone)]
pub struct Theme {
    pub background: Rgba,
    pub sidebar_background: Rgba,
    pub primary: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub border: Rgba,
    pub hover: Rgba,
    /// Status colors for inline error / success / warning text.
    pub danger: Rgba,
    pub success: Rgba,
    pub warning: Rgba,
}

impl Global for Theme {}

impl Theme {
    /// Compose a palette from a background tint family and an accent color —
    /// the two are independent settings.
    pub fn from_parts(tint: AppTint, accent: AccentColor) -> Self {
        // Neutrals shared by every tint.
        let base = Self {
            background: rgb(0x000000),
            sidebar_background: rgb(0x111111),
            primary: accent_rgba(accent),
            text: rgb(0xfafafa),
            text_muted: rgb(0xa1a1aa),
            border: rgb(0x232323),
            hover: rgb(0x232323),
            danger: rgb(0xef4444),
            success: rgb(0x22c55e),
            warning: rgb(0xf59e0b),
        };
        match tint {
            AppTint::Black => base,
            AppTint::Warm => Self {
                background: rgb(0x0a0908),
                sidebar_background: rgb(0x161412),
                border: rgb(0x2a2725),
                hover: rgb(0x2a2725),
                text_muted: rgb(0xa8a29e),
                ..base
            },
            AppTint::Zinc => Self {
                background: rgb(0x09090b),
                sidebar_background: rgb(0x18181b),
                border: rgb(0x27272a),
                hover: rgb(0x27272a),
                ..base
            },
            AppTint::Crimson => Self {
                background: rgb(0x0c0809),
                sidebar_background: rgb(0x1c1315),
                border: rgb(0x2f2225),
                hover: rgb(0x2f2225),
                text_muted: rgb(0xa89ea1),
                ..base
            },
            AppTint::Violet => Self {
                background: rgb(0x0a0810),
                sidebar_background: rgb(0x171226),
                border: rgb(0x2a2338),
                hover: rgb(0x2a2338),
                text_muted: rgb(0xa39fae),
                ..base
            },
        }
    }
}

fn accent_rgba(accent: AccentColor) -> Rgba {
    match accent {
        AccentColor::Starlight => rgb(0xffc107),
        AccentColor::Blue => rgb(0x3b82f6),
        AccentColor::Red => rgb(0xf43f5e),
        AccentColor::Purple => rgb(0xa855f7),
        AccentColor::Green => rgb(0x22c55e),
    }
}

pub fn init(cx: &mut App) {
    let settings = crate::settings::get(cx);
    apply(cx, settings.app_tint, settings.accent_color);
}

/// Install the preset's palette as the [`Theme`] global and push the whole
/// palette into gpui-component's theme too, so its widgets (buttons, inputs,
/// dropdowns, sidebar, title bar, …) follow the preset app-wide. The sidebar
/// and title bar are made transparent so they share the main window
/// background — the starfield shows through them. Refreshes all windows so a
/// mid-session switch repaints immediately.
pub fn apply(cx: &mut App, tint: AppTint, accent: AccentColor) {
    let palette = Theme::from_parts(tint, accent);
    let primary: Hsla = palette.primary.into();
    let background: Hsla = palette.background.into();
    let card: Hsla = palette.sidebar_background.into();
    let border: Hsla = palette.border.into();
    let hover: Hsla = palette.hover.into();
    let text: Hsla = palette.text.into();
    let text_muted: Hsla = palette.text_muted.into();
    let transparent = gpui::transparent_black();

    // Light accents (gold) need dark button text; darker ones read best white.
    let luminance =
        0.299 * palette.primary.r + 0.587 * palette.primary.g + 0.114 * palette.primary.b;
    let primary_foreground: Hsla = if luminance > 0.6 {
        rgb(0x0a0908).into()
    } else {
        rgb(0xfafafa).into()
    };

    let component_theme = gpui_component::Theme::global_mut(cx);
    let colors = &mut component_theme.colors;

    colors.background = background;
    colors.foreground = text;
    colors.border = border;
    colors.input = border;
    colors.ring = primary;

    let primary_hover = Hsla {
        l: (primary.l * 0.92).clamp(0.0, 1.0),
        ..primary
    };
    let primary_active = Hsla {
        l: (primary.l * 0.84).clamp(0.0, 1.0),
        ..primary
    };

    colors.primary = primary;
    colors.primary_hover = primary_hover;
    colors.primary_active = primary_active;
    colors.primary_foreground = primary_foreground;

    // Buttons have their own color family (they don't read `primary`/`secondary`).
    colors.button_primary = primary;
    colors.button_primary_hover = primary_hover;
    colors.button_primary_active = primary_active;
    colors.button_primary_foreground = primary_foreground;
    colors.button = card;
    colors.button_hover = hover;
    colors.button_active = hover;
    colors.button_foreground = text;

    colors.secondary = card;
    colors.secondary_hover = hover;
    colors.secondary_active = hover;
    colors.secondary_foreground = text;
    colors.accent = hover;
    colors.accent_foreground = text;
    colors.muted = hover;
    colors.muted_foreground = text_muted;
    colors.popover = card;
    colors.popover_foreground = text;
    colors.list = transparent;
    colors.list_hover = hover;
    colors.list_active = hover;
    colors.skeleton = hover;

    // Transparent chrome: the workspace paints the background (and the
    // starfield) behind these, so they blend into the main window.
    colors.sidebar = transparent;
    colors.sidebar_foreground = text;
    colors.sidebar_border = border;
    colors.sidebar_accent = hover;
    colors.sidebar_accent_foreground = text;
    colors.sidebar_primary = primary;
    colors.title_bar = transparent;
    colors.title_bar_border = border;

    // Newer components (Sidebar, TitleBar, …) render from `tokens`, which are
    // derived from `colors` — regenerate them or the changes above won't show.
    component_theme.tokens = gpui_component::ThemeTokens::from(&component_theme.colors);

    cx.set_global(palette);
    cx.refresh_windows();
}

pub const FONT_FAMILY: &str = ".SystemUIFont";

pub trait ThemeExt {
    fn theme(&self) -> &Theme;
}

impl<'a, V> ThemeExt for Context<'a, V> {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}
