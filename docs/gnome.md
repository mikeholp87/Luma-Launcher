# GNOME shortcuts

Open Luma Settings with `luma-launcher --settings`. Choose Register portal shortcut
to request Alt+Space through the compositor's consent interface. Availability is
checked by the actual portal call; installed portal files alone do not prove support.
The request is explicit. On success Luma saves the preference and registers again
on its next startup. The `shortcut` TOML field supplies the portal preferred trigger.

Fallback: Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts.
Add “Luma Launcher” with command `luma-launcher --toggle` and select Alt+Space.
GNOME may already use it for the window menu. Resolve that conflict explicitly in
Settings, or choose another combination. Luma does not change existing bindings.

Ensure the executable is in the session PATH. Enable autostart if warm activation
is desired immediately after login. Window placement and focus are compositor
decisions; no arbitrary Wayland positioning is promised.
