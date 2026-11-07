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
.luma .result-name {{ font-weight: 600; }}
.luma .search-bar {{ padding-bottom: 8px; border-bottom: 1px solid {border}; margin-bottom: 8px; }}
.luma .search-bar button {{ margin: 10px 4px; padding: 6px 10px; }}
.luma .description, .luma .footer {{ color: {muted}; font-size: 12px; }}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtk::{self as g, c};
    use glib::{prelude::*, translate::*};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn gtk_accepts_both_themes_and_custom_preferences() {
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
                unsafe {
                    let provider: glib::Object = from_glib_full(g::gtk_css_provider_new().cast());
                    let errors = Rc::new(RefCell::new(Vec::new()));
                    let captured = errors.clone();
                    provider.connect_local("parsing-error", false, move |values| {
                        captured
                            .borrow_mut()
                            .push(values[2].get::<glib::Error>().unwrap().to_string());
                        None
                    });
                    g::gtk_css_provider_load_from_data(
                        provider.as_ptr().cast(),
                        c(&stylesheet(&cfg, light)).as_ptr(),
                        -1,
                    );
                    assert!(errors.borrow().is_empty(), "{:?}", errors.borrow());
                }
            }
        }
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
