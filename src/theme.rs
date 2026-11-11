use luma_launcher::state::Config;

struct Palette {
    canvas: &'static str,
    surface: &'static str,
    text: &'static str,
    muted: &'static str,
    border: &'static str,
    selection: &'static str,
}

const LIGHT: Palette = Palette {
    canvas: "#f6f7fb",
    surface: "#ffffff",
    text: "#202330",
    muted: "#626a7d",
    border: "#d5dbea",
    selection: "#e1e7f8",
};

const DARK: Palette = Palette {
    canvas: "#20222b",
    surface: "#282b36",
    text: "#edf0f8",
    muted: "#9ba4b8",
    border: "#4a5062",
    selection: "#333c58",
};

/// Font size the stylesheet's fixed sizes were designed around.
pub const BASE_FONT: i32 = 19;
/// Scale a pixel size designed for `BASE_FONT` to the configured font size.
pub fn scaled(font_size: i32, base_px: i32, min_px: i32) -> i32 {
    ((base_px as f32 * font_size as f32 / BASE_FONT as f32).round() as i32).max(min_px)
}

pub fn stylesheet(cfg: &Config, light: bool) -> String {
    let Palette {
        canvas,
        surface,
        text,
        muted,
        border,
        selection,
    } = if light { LIGHT } else { DARK };
    let accent = &cfg.accent;
    let font_size = cfg.font_size;
    let row_padding = if cfg.compact { 6 } else { 10 };
    let name_px = scaled(font_size, 15, 11);
    let small_px = scaled(font_size, 12, 10);

    format!(
        r#"
window.luma {{ background: {canvas}; color: {text}; }}
window.luma.launcher {{ background: transparent; }}
.luma .palette {{ background: {canvas}; border: 1px solid {border}; border-radius: 16px; padding: 10px; }}
.luma label, .luma image {{ color: {text}; }}
.luma list, .luma scrolledwindow, .luma viewport {{ background: transparent; color: {text}; }}
.luma row {{ background: transparent; color: {text}; padding: {row_padding}px 12px; border-left: 3px solid transparent; border-radius: 9px; }}
.luma row:hover {{ background: {surface}; }}
.luma row:selected, .luma row:selected:hover {{ background: {selection}; color: {text}; border-left-color: {accent}; outline: none; }}
.luma .result-name {{ font-weight: 600; font-size: {name_px}px; }}
.luma .search-bar {{ padding-bottom: 8px; border-bottom: 1px solid {border}; margin-bottom: 8px; }}
.luma .search-bar button {{ margin: 10px 4px; padding: 6px 10px; }}
.luma .description, .luma .footer {{ color: {muted}; font-size: {small_px}px; }}
.luma row:selected label {{ color: {text}; }}
.luma .footer {{ padding: 12px; border-top: 1px solid {border}; }}
.luma .settings {{ padding: 20px 24px; }}
.luma .settings-title {{ font-size: 24px; font-weight: 700; }}
.luma .settings-help {{ color: {muted}; font-size: 12px; }}
.luma .settings-section {{ font-size: 15px; font-weight: 700; padding-top: 16px; padding-bottom: 4px; }}
.luma .settings checkbutton {{ padding: 8px 2px; color: {text}; }}
.luma .settings checkbutton check {{ background: {surface}; color: {text}; border: 1px solid {muted}; border-radius: 4px; min-width: 18px; min-height: 18px; margin-right: 10px; }}
.luma .settings checkbutton check:checked {{ background: {selection}; color: {text}; border-color: {muted}; }}
.luma .settings checkbutton:focus-visible {{ outline: 2px solid {muted}; outline-offset: 2px; }}
.luma entry {{ background: {surface}; color: {text}; border: 1px solid {border}; border-radius: 8px; box-shadow: none; padding: 8px 10px; }}
.luma entry text {{ color: {text}; caret-color: {text}; }}
.luma entry.search {{ background: transparent; border-color: transparent; padding: 16px 12px; font-size: {font_size}px; }}
.luma entry placeholder, .luma entry image {{ color: {muted}; opacity: 1; }}
.luma entry selection {{ background: {selection}; color: {text}; }}
.luma textview.keywords {{ border: 1px solid {border}; border-radius: 8px; padding: 8px 10px; font-family: monospace; font-size: 12px; }}
.luma textview.keywords, .luma textview.keywords text {{ background: {surface}; color: {text}; caret-color: {text}; }}
.luma textview.keywords text selection {{ background: {selection}; color: {text}; }}
.luma textview.keywords:focus-within {{ outline: 2px solid {muted}; outline-offset: 2px; }}
.luma button {{ background: {surface}; color: {text}; border: 1px solid {border}; border-radius: 8px; box-shadow: none; text-shadow: none; padding: 8px 12px; }}
.luma button:hover {{ background: {selection}; border-color: {muted}; }}
.luma button:active {{ background: {selection}; border-color: {accent}; }}
.luma button:disabled, .luma button:disabled label {{ color: {muted}; }}
.luma entry:focus-within, .luma button:focus-visible, .luma row:focus-visible {{ outline: 2px solid {muted}; outline-offset: 2px; }}
.luma headerbar {{ background: {canvas}; color: {text}; box-shadow: none; border-bottom: 1px solid {border}; }}
.luma headerbar button {{ background: transparent; border-color: transparent; }}
.luma headerbar button:hover {{ background: {selection}; }}
.luma scrollbar, .luma scrollbar trough {{ background: {canvas}; border: none; }}
.luma scrollbar slider {{ background: {border}; border: 1px solid {muted}; border-radius: 9px; margin: 0; padding: 0; min-width: 6px; min-height: 6px; }}
.luma scrollbar slider:hover {{ background: {muted}; }}
.luma popover contents {{ background: {surface}; color: {text}; border: 1px solid {border}; }}
"#
    )
}

/// Every theme/preference combination must parse without GTK CSS errors.
/// Needs an initialised GTK, so the single GTK test in `ui.rs` calls it.
#[cfg(test)]
pub(crate) fn assert_stylesheets_parse() {
    use gtk4 as gtk;
    use std::{cell::RefCell, rc::Rc};
    for light in [false, true] {
        for cfg in [
            Config::default(),
            Config {
                accent: "#12abef".into(),
                font_size: 32,
                compact: true,
                ..Config::default()
            },
        ] {
            let provider = gtk::CssProvider::new();
            let errors = Rc::new(RefCell::new(Vec::new()));
            let captured = errors.clone();
            provider.connect_parsing_error(move |_, _, error| {
                captured.borrow_mut().push(error.to_string());
            });
            provider.load_from_data(&stylesheet(&cfg, light));
            assert!(errors.borrow().is_empty(), "{:?}", errors.borrow());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_size_scales_every_launcher_text_size() {
        let at = |font_size| {
            stylesheet(
                &Config {
                    font_size,
                    ..Config::default()
                },
                false,
            )
        };
        // Defaults are unchanged from the original fixed sizes.
        let default = at(BASE_FONT);
        assert!(default.contains("font-size: 19px"));
        assert!(default.contains("font-weight: 600; font-size: 15px"));
        assert!(default.contains("font-size: 12px; }"));
        let large = at(32);
        assert!(large.contains("entry.search") && large.contains("font-size: 32px"));
        assert!(large.contains("font-weight: 600; font-size: 25px"));
        assert!(large.contains("font-size: 20px; }"));
        let small = at(12);
        assert!(small.contains("font-weight: 600; font-size: 11px"));
        assert!(small.contains("font-size: 10px; }"));
        assert_eq!(scaled(32, 36, 24), 61);
    }

    #[test]
    fn scrollbar_slider_cannot_go_negative() {
        // Theme margins plus our border once summed below zero (GTK warns
        // "reported min width -4"); the slider must reset them.
        let css = stylesheet(&Config::default(), false);
        let rule = css
            .lines()
            .find(|l| l.contains("scrollbar slider {"))
            .unwrap();
        assert!(
            rule.contains("margin: 0") && rule.contains("padding: 0"),
            "{rule}"
        );
    }

    #[test]
    fn text_remains_readable_on_normal_and_selected_surfaces() {
        fn luminance(hex: &str) -> f64 {
            let rgb = [1, 3, 5].map(|start| {
                let value = u8::from_str_radix(&hex[start..start + 2], 16).unwrap() as f64 / 255.;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            });
            rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722
        }
        for palette in [LIGHT, DARK] {
            for (fg, bg) in [
                (palette.text, palette.canvas),
                (palette.text, palette.surface),
                (palette.muted, palette.canvas),
                (palette.muted, palette.surface),
                (palette.text, palette.selection),
            ] {
                let (fg_lum, bg_lum) = (luminance(fg), luminance(bg));
                let contrast = (fg_lum.max(bg_lum) + 0.05) / (fg_lum.min(bg_lum) + 0.05);
                assert!(contrast >= 4.5, "{fg} on {bg}: {contrast:.2}:1");
            }
        }
    }
}
