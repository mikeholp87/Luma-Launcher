//! Minimal GTK4 ABI used by the offline build. All calls stay on the GTK thread.
//! Objects are owned by GTK parents or held through GLib reference-counted objects.
use std::ffi::{c_char, c_int, c_uint, c_void, CString};
pub type Widget = *mut c_void;
unsafe extern "C" {
    pub fn gtk_application_new(id: *const c_char, flags: c_uint) -> Widget;
    pub fn gtk_application_window_new(app: Widget) -> Widget;
    pub fn gtk_window_set_startup_id(w: Widget, id: *const c_char);
    pub fn gtk_window_set_title(w: Widget, title: *const c_char);
    pub fn gtk_window_set_default_size(w: Widget, width: c_int, height: c_int);
    pub fn gtk_window_set_decorated(w: Widget, b: c_int);
    pub fn gtk_window_set_child(w: Widget, child: Widget);
    pub fn gtk_window_destroy(w: Widget);
    pub fn gtk_window_present(w: Widget);
    pub fn gtk_widget_set_visible(w: Widget, b: c_int);
    pub fn gtk_widget_grab_focus(w: Widget) -> c_int;
    pub fn gtk_widget_get_first_child(w: Widget) -> Widget;
    #[cfg(test)]
    pub fn gtk_widget_get_next_sibling(w: Widget) -> Widget;
    pub fn gtk_widget_add_css_class(w: Widget, class: *const c_char);
    pub fn gtk_widget_set_hexpand(w: Widget, b: c_int);
    pub fn gtk_widget_set_tooltip_text(w: Widget, text: *const c_char);
    pub fn gtk_box_new(orientation: c_int, spacing: c_int) -> Widget;
    pub fn gtk_box_append(w: Widget, child: Widget);
    pub fn gtk_entry_set_max_length(w: Widget, max: i32);
    pub fn gtk_entry_new() -> Widget;
    pub fn gtk_entry_set_placeholder_text(w: Widget, text: *const c_char);
    pub fn gtk_entry_set_icon_from_icon_name(w: Widget, pos: c_int, name: *const c_char);
    pub fn gtk_editable_get_text(w: Widget) -> *const c_char;
    pub fn gtk_editable_set_text(w: Widget, text: *const c_char);
    pub fn gtk_label_new(text: *const c_char) -> Widget;
    pub fn gtk_label_set_text(w: Widget, text: *const c_char);
    pub fn gtk_label_set_xalign(w: Widget, x: f32);
    pub fn gtk_label_set_ellipsize(w: Widget, mode: c_int);
    pub fn gtk_label_set_wrap(w: Widget, wrap: c_int);
    pub fn gtk_label_set_mnemonic_widget(w: Widget, target: Widget);
    pub fn gtk_check_button_new_with_label(text: *const c_char) -> Widget;
    pub fn gtk_check_button_set_active(w: Widget, active: c_int);
    pub fn gtk_list_box_new() -> Widget;
    pub fn gtk_list_box_append(w: Widget, child: Widget);
    pub fn gtk_list_box_remove(w: Widget, child: Widget);
    pub fn gtk_list_box_get_row_at_index(w: Widget, index: c_int) -> Widget;
    pub fn gtk_list_box_select_row(w: Widget, row: Widget);
    pub fn gtk_list_box_row_get_index(w: Widget) -> c_int;
    pub fn gtk_scrolled_window_new() -> Widget;
    pub fn gtk_scrolled_window_set_child(w: Widget, child: Widget);
    pub fn gtk_scrolled_window_set_max_content_height(w: Widget, height: c_int);
    pub fn gtk_widget_measure(
        w: Widget,
        orientation: c_int,
        for_size: c_int,
        minimum: *mut c_int,
        natural: *mut c_int,
        minimum_baseline: *mut c_int,
        natural_baseline: *mut c_int,
    );
    pub fn gtk_scrolled_window_set_propagate_natural_height(w: Widget, b: c_int);
    pub fn gtk_image_new_from_gicon(icon: Widget) -> Widget;
    pub fn gtk_image_set_pixel_size(w: Widget, size: c_int);
    pub fn gtk_event_controller_set_propagation_phase(w: Widget, phase: c_int);
    pub fn gtk_event_controller_key_new() -> Widget;
    pub fn gtk_widget_add_controller(w: Widget, controller: Widget);
    pub fn gtk_css_provider_new() -> Widget;
    pub fn gtk_css_provider_load_from_data(w: Widget, data: *const c_char, len: isize);
    pub fn gtk_style_context_add_provider_for_display(
        display: Widget,
        provider: Widget,
        priority: c_uint,
    );
    pub fn gtk_widget_get_display(w: Widget) -> Widget;
    pub fn gdk_display_get_app_launch_context(display: Widget) -> Widget;
    pub fn gtk_widget_get_clipboard(w: Widget) -> Widget;
    pub fn gdk_clipboard_set_text(w: Widget, text: *const c_char);
    pub fn gtk_get_major_version() -> c_uint;
    pub fn gtk_get_minor_version() -> c_uint;
}
pub fn c(s: &str) -> CString {
    CString::new(s.replace('\0', "�")).expect("NUL removed")
}
