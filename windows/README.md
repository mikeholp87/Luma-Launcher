# Luma for Windows

The compiled WPF app in `native/` requires the .NET 10 Windows Desktop Runtime.
Build with the .NET 10 SDK:

```powershell
dotnet publish .\windows\native\LumaLauncher.csproj -c Release -o .\dist\LumaLauncher
.\dist\LumaLauncher\LumaLauncher.exe
```

Keep the entire published folder together, including the Unicode license.
The per-user install is `%LOCALAPPDATA%\Programs\LumaLauncher`, with a Start Menu
shortcut. Publishing itself does not install. The older PowerShell preview is
separate and does not share settings.

## Search and controls

Start Menu shortcuts are indexed in the background with their descriptions,
category folders and target executable names. Start Menu changes automatically
refresh the index. Search uses Unicode full case folding and compatibility
normalization. Matching tiers are exact name, prefix, word start, substring,
acronym, fuzzy subsequence, keywords, then optional descriptions. Tighter fuzzy
matches win; favorites, logarithmic frequency buckets and recency break ties.
History records only Luma launches and retains at most 1,000 applications.

| Key | Action |
|---|---|
| Ctrl+Alt+Space (configurable) | Toggle palette globally |
| Ctrl+Alt+S (configurable) | Open Settings globally |
| Enter | Open selection or URL; copy calculation |
| Up/Down, Ctrl+K/J, Ctrl+P/N | Previous/next result |
| Ctrl+1 through Ctrl+9 | Open the corresponding visible application |
| Escape | Clear search; press again to hide |
| Ctrl+D / Ctrl+Delete | Pin/unpin / forget application history |
| Ctrl+Enter | Windows shell actions and shortcut location |
| Ctrl+comma / Ctrl+plus | Open Settings, including numpad plus |
| F5 | Refresh applications |
| Alt+F4 or quit button | Exit resident process |

The palette hides after launching and on focus loss. Settings and action dialogs
remain usable. Palette height follows results up to a scrolling cap. Font size
scales search, result names, descriptions, footer and icons.
The palette opens centered in the usable area of the monitor containing the
pointer and stays centered as results or appearance settings change its size.

Use `= (250 + 75) * 2` for arithmetic, `g terms` or `ddg terms` for web search,
an explicit HTTP/HTTPS URL, or a bare domain such as `github.com/rust-lang`.
Bare domains accept `.com .org .net .io .dev .app .ai .edu .gov`; filenames and
ordinary app IDs remain searches. URL credentials and executable schemes are
rejected. Arithmetic supports unary signs and bounded nesting, without code
evaluation; results such as `0.1+0.2` display cleanly as `0.3`.

## Settings and privacy

Text fields save on Enter or focus loss. Checkboxes and themes save immediately.
Closing Settings saves pending fields; Save settings retries edits and closes
after success. Invalid input or save failures retain edits and show an error.
Failed checkbox/theme writes restore the saved selection. Previously saved
changes remain saved when Settings closes. History clearing and personalization
reset require a second click and apply only after a successful save.
Escape or Discard pending edits closes without retrying unsaved fields; changes
already saved remain in effect.

Preferences cover both global shortcuts, theme, accent, width, result limit,
font size, compact layout, the Settings button, history, descriptions, fuzzy
matching, clearing search on opening and login startup. System theme follows
Windows appearance notifications while running. Conflicting shortcuts restore
previous registrations when possible.

Preferences and personalization live in
`%LOCALAPPDATA%\LumaLauncher\native-settings.json`. Writes replace the file
atomically. Corrupt or oversized files are preserved and saving is disabled until
repaired. No telemetry or tracking outside Luma. Disabling history stops new
records; clear existing records separately. Login startup manages only Luma's
current-user Windows Run value; re-enable it after moving the executable.

## Resident commands

Pass one flag at a time. No flag or `--show` shows the palette. The following
commands reach the existing instance through a current-user/session named pipe:

- `--hide`, `--toggle`, `--settings`, `--refresh`, `--quit`.
- `--background` starts hidden and leaves an existing instance alone.
- `--version`, `--diagnostics`, `--help` print information without a running app.
- `--self-test` runs core and settings regression checks.

Use `dotnet .\dist\LumaLauncher\LumaLauncher.dll --diagnostics` for console output;
the executable is a GUI application. Starting the executable again shows the
existing palette; use the global shortcut or `--toggle` to toggle it.

## Platform differences and validation

See [the pinned Linux feature comparison](linux-feature-parity.md). Windows
indexes `.lnk` shortcuts; apps without them, including some Store apps, may not
appear. Descriptions may be empty. Icons use the Windows shell with a generic
fallback. Desktop actions map to Windows shell verbs. KDE/GNOME usage databases,
XDG portals, D-Bus and Linux packaging stay Linux-specific. Windows controls
foreground activation and reserves some global shortcuts.

```powershell
dotnet build .\windows\native\LumaLauncher.csproj -c Release
dotnet .\windows\native\bin\Release\net10.0-windows\LumaLauncher.dll --self-test
```

Checks cover ranking tiers, Unicode, URL/calculator safety, keyboard mappings,
CLI parsing, shortcut rollback, persistence, failed writes, autosave, pending
edits on close and destructive-button confirmation. Real desktop focus, theme
notifications and actual shortcut conflicts need live Windows verification.

To remove Luma, disable login startup, quit, and remove the install folder and
Start Menu shortcut. Delete the data folder separately only to remove preferences
and history. Unicode data in `native/Unicode/` uses the included Unicode license.
