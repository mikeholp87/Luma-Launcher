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
- Regular application launches send child stdout/stderr to
  `$XDG_STATE_HOME/luma-launcher/applications.log` (default
  `~/.local/state/luma-launcher/applications.log`) with mode 0600 on creation.
  This keeps application output, such as Electron GPU warnings, out of Luma's
  terminal. At 5 MiB, the log rotates to `applications.log.1` on the next launch;
  only one prior log is retained. A single verbose app can exceed the limit before
  the next launch. If the log cannot be opened, Luma warns
  once for that launch and discards child output so the app can still start.
  Launch errors still appear in Luma. Desktop actions and browser handlers keep
  their native GIO launch paths; D-Bus-activated services manage their own output.
  Apps that are already running retain their original output streams; close and
  reopen them after updating Luma to use the new log destination.

Known gaps are listed in verification.md. Theme/HiDPI/focus problems need a
reproduction on the actual compositor; headless compilation does not validate them.
