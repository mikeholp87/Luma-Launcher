# Troubleshooting

- Run `luma-launcher --diagnostics` to report GTK version, desktop/session,
  configuration path, bus reachability, and application count without opening UI.
- “Failed to open display” means the display socket is inaccessible or the
  session environment is wrong. Run inside your graphical login session.
- A session-bus error prevents graphical startup. Do not launch with sudo or
  reuse another user's bus address. Restart from the desktop session.
- If apps are missing, check their desktop-entry visibility, TryExec, and
  XDG_DATA_DIRS (including Flatpak/Snap exports), then run `--refresh`.
- Missing icons fall back to the executable theme icon. Install your desktop's
  icon theme if icons are blank.
- Portal denial or missing interface: use the desktop-managed shortcut fallback.
  Retry registration in Settings after a failure, closure, or timeout. Never remove unrelated shortcuts.
- Repair corrupt TOML before restarting. In-memory defaults do not overwrite it.
- Logs are emitted to stderr as level/message fields; redirect them when diagnosing.

Known gaps are listed in verification.md. Theme/HiDPI/focus problems need a
reproduction on the actual compositor; headless compilation does not validate them.
