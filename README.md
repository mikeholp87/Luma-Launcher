# Luma Launcher

<img src="assets/io.github.luma_launcher.LumaLauncher.svg" alt="Luma Launcher icon" width="88" height="88">

**A native, keyboard-driven Linux application launcher built with Rust and GTK4.**

Luma brings installed applications, local favorites, quick calculations, and web
search into a floating search palette. Assign a desktop shortcut, type what you
need, and press Enter. The resident process keeps the application index ready
between activations.

```text
Alt+Space → Search applications → Enter
```

Designed for KDE Plasma and GNOME, with Wayland-compatible desktop integration
and an X11-compatible GTK interface. Luma uses GIO application discovery and
launching instead of interpreting desktop-entry commands itself.

> **Project status: initial release, 0.1.0.** Release builds, automated tests,
> local installer checks, and Debian package extraction pass. Interactive
> KDE/GNOME behavior, compositor focus, real global shortcuts, and Debian Stable
> installation still need verification. See the [verification report](docs/verification.md).

## Features

- **Application search:** exact, prefix, substring, fuzzy subsequence, acronym,
  keyword, generic-name, and optional description matching.
- **Unicode support:** case folding and normalization, including composed and
  decomposed characters.
- **Native application metadata:** names, descriptions, themed icons, desktop
  actions, visibility rules, and desktop-ID precedence through GIO.
- **Keyboard navigation:** select results, launch, dismiss, pin favorites, remove
  history entries, and open settings without reaching for the mouse.
- **Personalized ranking:** favorites, recent launches, and launch frequency,
  while preserving the priority of exact name matches.
- **Resident activation:** show, hide, toggle, and refresh commands communicate
  with one application instance through the desktop session bus.
- **Event-driven discovery:** application-change notifications refresh the index
  without continuous directory scanning.
- **Appearance controls:** dark, light, and system theme modes; accent color,
  width, result count, font size, and compact layout settings.
- **Quick tools:** bounded arithmetic with clipboard copying, Google/DuckDuckGo
  prefixes, explicit HTTP/HTTPS URL opening, and declared desktop actions.
- **Local privacy:** no telemetry, advertising, downloaded application icons, or
  tracking of activity outside Luma. Launch history can be disabled or cleared.

## Requirements

| Component | Requirement |
|---|---|
| Operating system | Linux; Debian Stable and Testing are the intended package targets |
| Desktop | A graphical GTK4-compatible session with session D-Bus |
| Toolchain | Current stable Rust and Cargo; development used Rust 1.96 |
| Native libraries | GTK4 runtime, GLib/GIO, a C linker, and pkg-config |
| Packaging tools | dpkg-dev and desktop-file-utils for Debian packaging |

The interface currently uses a small GTK4 ABI module and Rust GIO/GLib bindings.
A migration to gtk-rs is planned when dependencies can be fetched and validated.
The cached GLib dependency includes an explicit upstream security backport;
[provenance and maintenance notes](vendor/README.md) are included.

## Build from source

On Debian, install the development prerequisites:

```bash
sudo apt update
sudo apt install build-essential pkg-config libglib2.0-dev libgtk-4-dev
```

Install a stable Rust toolchain if it is not already available, then:

```bash
git clone https://github.com/mikeholp87/Luma-Launcher.git
cd Luma-Launcher
cargo build --locked
cargo test --locked
cargo build --locked --release
./target/release/luma-launcher --show
```

Run graphical commands from your normal desktop session, without sudo. If Cargo's
registry cache is already populated, add `--offline` to build and test commands.
The application itself does not require network access for application search.
Web searches and URLs intentionally open the user's browser.

## Install locally without root

After building the release executable:

```bash
bash scripts/install.sh
```

The installer writes the executable, desktop entry, and icon to:

```text
~/.local/bin/luma-launcher
~/.local/share/applications/io.github.luma_launcher.LumaLauncher.desktop
~/.local/share/icons/hicolor/scalable/apps/io.github.luma_launcher.LumaLauncher.svg
```

It honors `XDG_DATA_HOME`; `LUMA_BIN_DIR` overrides the binary destination and must
be absolute. Installed desktop entries contain an absolute executable path, so
application-menu launching does not depend on the desktop's PATH. Add
`~/.local/bin` to your shell PATH to use `luma-launcher` from a terminal.
Executable paths containing percent, equals, or control characters are rejected
before installation because of desktop-entry parsing limitations.

### Start at login

Autostart is off initially. Enable it from Settings or run:

```bash
bash scripts/autostart.sh enable
```

The script requires `luma-launcher` in PATH during setup and stores its resolved
absolute path. Disable it with `bash scripts/autostart.sh disable`. Only Luma's
own XDG autostart entry is managed.

### Build a Debian package

```bash
sudo apt install dpkg-dev desktop-file-utils
bash scripts/package.sh
```

On an amd64 build host, the output is:

```text
dist/luma-launcher_0.1.0_amd64.deb
```

Install the locally built package with:

```bash
sudo apt install ./dist/luma-launcher_0.1.0_amd64.deb
```

Dependencies are generated from the executable's actual shared-library usage.
A package built on Testing/Unstable is not automatically verified for Stable.
A Debian Stable [container recipe](packaging/Containerfile) and
[CI workflow](.github/workflows/check.yml) are supplied, but have not been executed
in the restricted development environment. No hosted binary release is assumed.

## Assign a global shortcut

**Alt+Space is the preferred binding; installation does not claim it automatically.**

Use this command for the shortcut:

```bash
/home/YOUR_USERNAME/.local/bin/luma-launcher --toggle
```

Replace the path with your actual installation location. `--toggle` shows the
palette when hidden and closes it when visible.

### KDE Plasma

Open **System Settings → Keyboard → Shortcuts**. Add Luma Launcher as an
application or command and assign Alt+Space to its activation command. If another
action owns the shortcut, review the conflict and explicitly reassign it, or
choose an unused combination. Luma does not edit existing KDE assignments.

### GNOME

Open **Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts**.
Add Luma Launcher with the toggle command and assign your preferred combination.
GNOME may already use Alt+Space for the window menu; resolve that conflict in
Settings before using it for Luma.

### Desktop portal

Choose **Register portal shortcut** in Luma Settings to request a binding through
the XDG GlobalShortcuts portal. The desktop may ask for consent or offer a
shortcut-selection dialog. Actual backend availability is checked by the request;
portal packages being installed does not establish support. Desktop-managed
shortcuts remain the fallback.

Wayland controls window placement and focus. Luma does not intercept unrelated
keystrokes or bypass compositor restrictions. See the dedicated
[KDE](docs/kde.md) and [GNOME](docs/gnome.md) guides.

## Keyboard controls

| Key | Action |
|---|---|
| Configured global shortcut | Toggle Luma through the desktop or portal |
| Enter | Launch the selected application, open a URL, or copy a calculation |
| Escape | Hide the palette |
| Up / Down | Select the previous or next application |
| Ctrl+K / Ctrl+J | Select the previous or next application |
| Ctrl+A | Select search text |
| Ctrl+, or Ctrl++ | Open Settings |
| Ctrl+D | Pin or unpin the selected application |
| Ctrl+Delete | Remove the selected application's launch history |
| Ctrl+Enter | Show declared desktop actions |

The palette hides after successful launch and when its window loses focus.
Typing should begin in the search field immediately; compositor-specific focus
behavior still needs live desktop verification.

## Quick tools

| Input | Result |
|---|---|
| `= 25 * 4` | Calculate; Enter copies `100` |
| `= (250 + 75) * 2` | Calculate with parentheses |
| `g best linux utilities` | Search Google in the default browser |
| `ddg rust gtk4 tutorial` | Search DuckDuckGo in the default browser |
| `https://www.rust-lang.org/` | Offer to open the URL in the default browser |

The calculator accepts basic arithmetic and bounded nesting, rejects invalid or
nonfinite results, and never evaluates executable code. Arbitrary shell-command
mode is not implemented. Ordinary application-search input cannot run shell commands.

## Command-line interface

| Command | Behavior |
|---|---|
| `luma-launcher` or `--show` | Show the launcher |
| `luma-launcher --hide` | Hide the launcher |
| `luma-launcher --toggle` | Toggle the launcher |
| `luma-launcher --settings` | Open Settings |
| `luma-launcher --refresh` | Refresh application discovery |
| `luma-launcher --background` | Start resident without showing the palette |
| `luma-launcher --diagnostics` | Print desktop, GTK, Rust, paths, bus status, and app count |
| `luma-launcher --version` | Print the application version |
| `luma-launcher --quit` | Stop the resident instance |

Pass one command flag at a time. Version and diagnostics work without a display.
Other commands require a graphical session and reachable session D-Bus.

## Configuration and data

| Data | Default location | Override |
|---|---|---|
| Preferences | `~/.config/luma-launcher/config.toml` | `XDG_CONFIG_HOME` |
| Favorites and launch history | `~/.local/state/luma-launcher/history.toml` | `XDG_STATE_HOME` |
| Login entry | `~/.config/autostart/io.github.luma_launcher.LumaLauncher.desktop` | `XDG_CONFIG_HOME` |

Example preferences, showing a subset of supported settings:

```toml
theme = "dark"
accent = "#8da9ff"
width = 680
results = 8
font_size = 19
compact = false
history = true
descriptions = true
fuzzy = true
clear_on_show = true
portal_shortcut = false
shortcut = "ALT+space"
```

Themes are `dark`, `light`, or `system`. Width accepts 360–1200 pixels, font size
12–32, and result count 1–20. Accent colors use `#RRGGBB`. Press Enter after editing
a Settings field to save it. Restart Luma after editing TOML externally.

History only records applications launched through Luma. Disabling history stops
new records; clear existing records separately in Settings. Writes use exclusive
temporary files, atomic replacement, and private state-file permissions. Corrupt
files are preserved rather than silently overwritten. Repair them and restart.

## Tests and verification

```bash
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
cargo test --locked --release --test glib_variant
bash scripts/test-install.sh
```

`bash scripts/test.sh` runs the same core checks using the offline cache.
Shell scripts are also checked with ShellCheck and shfmt.

The pre-publication source snapshot has **30 passing tests and one ignored graphical test**.
Coverage includes matching/ranking, Unicode, input limits, config corruption,
concurrent persistence, portal response parsing, keyboard-controller setup, theme CSS parsing/contrast,
calculator/URL safety, GIO launching, XDG precedence, and installer path quoting.
The vendored upstream dependency retains 34 existing Rust style/lifetime warnings;
application code passes strict Clippy.

Actual KDE/GNOME shortcut registration, multi-monitor/HiDPI behavior, suspend/resume,
resident resource use, and Stable package installation remain unverified.
A complete dependency-advisory scan is also outstanding. See the
[audit](docs/audit.md), [security review](docs/security.md), and
[acceptance status](docs/verification.md).

## Performance measurements

The recorded release-mode synthetic benchmark on an Intel Core 7 150U used
100 searches per dataset, query `app 99`, and eight results:

| Indexed entries | Search p50 | Search p95 |
|---:|---:|---:|
| 100 | 91 µs | 118 µs |
| 1,000 | 510 µs | 840 µs |
| 10,000 | 4,846 µs | 5,537 µs |

These are matching/sorting measurements from a development snapshot, not GTK
rendering or global-shortcut activation times. They do not establish a warm
activation, idle CPU, or resident-memory guarantee. Reproduce search measurements
with `cargo bench --bench search`; [methodology and limitations](docs/performance.md)
include the separate Hyperfine CLI measurements.

## Project layout

```text
src/          GTK interface, desktop integration, search, state, and quick tools
tests/        CLI, GIO, installation, and dependency regression tests
benches/      Synthetic search benchmark
assets/       Application icon
scripts/      Build checks, local installation/removal, autostart, and packaging
packaging/    Desktop entry and Debian Stable container recipe
debian/       Debian package metadata
docs/         Architecture, setup, security, audits, and test reports
vendor/       Documented GLib security backport and upstream licensing
```

Read the [architecture guide](docs/architecture.md) for lifecycle and data flow.
Future work is tracked in the verification report: maintained gtk-rs bindings,
real desktop integration tests, Stable package validation, richer settings, and
search/rendering refinements justified by measurements.

## Troubleshooting

- **Cannot access session D-Bus:** run inside your graphical login session as
  your normal user. Do not launch through sudo or reuse another user's bus address.
- **Failed to open display:** check the desktop session environment and display
  permissions; headless or sandboxed shells may not be allowed to open windows.
- **Shortcut unavailable:** retry portal setup after a failure or configure the
  desktop-managed toggle command. Check for existing assignments.
- **Missing applications:** confirm desktop-entry visibility, executable
  availability, and XDG data paths, including Flatpak/Snap export directories.
  Run `luma-launcher --refresh` after correcting the environment.
- **Preferences do not save:** inspect the Settings status message and repair
  malformed TOML or unwritable paths before restarting.

Run `luma-launcher --diagnostics` when reporting a problem. More detail is in the
[troubleshooting guide](docs/troubleshooting.md).

## Uninstall

For a local installation:

```bash
luma-launcher --quit
bash scripts/uninstall.sh
```

Preferences are preserved. To explicitly remove Luma's preferences and history:

```bash
bash scripts/uninstall.sh --purge
```

For a Debian package installation, remove the package through the package manager.
Remove user-created shortcut assignments manually; uninstalling does not alter
unrelated desktop shortcuts or application settings.

## Contributing

Keep changes focused and preserve keyboard accessibility, GIO launch semantics,
and local-only data handling. Include a regression test for nontrivial fixes and
report which desktop/session combinations you actually exercised. UI changes
should include screenshots when a graphical environment is available.

Before sending a pull request, run the checks above, lint modified shell scripts,
and scan for secrets. Describe the user-visible behavior, validation performed,
and any remaining platform limitations. Existing environment-specific contributor
notes are in [AGENTS.md](AGENTS.md).

## License

Luma Launcher is available under the [MIT License](LICENSE). Vendored GLib retains
its [upstream license](vendor/glib/LICENSE), copyright notices, and documented
security-patch provenance.
