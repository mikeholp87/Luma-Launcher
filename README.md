# Luma Launcher

<img src="assets/io.github.luma_launcher.LumaLauncher.svg" alt="Luma Launcher icon" width="88" height="88">

**A keyboard-driven application launcher for Linux and Windows.**

The Linux application is built with Rust and GTK4. The Windows application
uses .NET and WPF, with a similar search palette and
keyboard-driven workflow. The two implementations have different features;
the details below describe Linux unless explicitly marked Windows.

## Windows application

Build the native Windows app with the .NET 10 SDK, then run the executable:

```powershell
dotnet publish .\windows\native\LumaLauncher.csproj -c Release -o .\dist\LumaLauncher
.\dist\LumaLauncher\LumaLauncher.exe
```

Keep the published files together; the app requires the .NET 10 Windows Desktop
Runtime. No Rust, GTK, or administrator access is required. Ctrl+Alt+Space
toggles the palette when available;
running the launcher again shows it (`--toggle` toggles it). Search Start Menu shortcuts, use
arrow keys and Enter to open a result, and press Escape to hide the palette.
F5 refreshes the index. The quit button or Alt+F4 exits the resident process.

Windows supports Unicode search with exact, prefix, word-start, substring,
acronym, fuzzy, keyword and description matching; favorites and frequency/recency
ranking; automatic indexing; local history; a calculator; bare-domain URLs; and
settings for global shortcuts, theme, accent, sizing, search, and startup.
Ctrl+D pins or unpins, Ctrl+Delete forgets an app's history, Ctrl+J/K
navigate, and Ctrl+, opens Settings. Ctrl+N/P also navigate, Ctrl+1–9 launches
visible apps, and Ctrl+Enter opens Windows shell actions. Settings save automatically.
Ctrl+Alt+S opens Settings globally by
default; both global shortcuts can be changed in Settings. Try `= 25 * 4`, `g search terms`, or
`ddg search terms`. History and settings are stored locally in
`%LOCALAPPDATA%\LumaLauncher\native-settings.json`.

The app indexes `.lnk` shortcuts in the user and common Start Menu Programs
folders. Some Microsoft Store apps and other apps without shortcuts there will
not appear. KDE/GNOME system usage ranking and Linux desktop actions stay
platform-specific. The core logic has been checked, but interactive Windows UI and global
shortcut behavior still need verification. See the [Windows guide](windows/README.md)
for commands, privacy, troubleshooting, and tests.

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
> local installer checks, and the apt package pass; the package installs and runs on
> Debian testing with KDE Plasma (Wayland). GNOME behavior, compositor focus on other
> desktops, real global shortcuts, and Debian 13 installation still need verification.
> See the [verification report](docs/verification.md).

## Install (Debian, Ubuntu and derivatives)

The quickest way is Luma's signed apt repository. Updates then arrive with your normal
`sudo apt update && sudo apt upgrade`.

**Supported:** Debian 13 (trixie) or newer and Ubuntu 24.04 or newer, on amd64, and
distributions based on them (for example Linux Mint 22). Older releases such as Ubuntu
22.04, Debian 12 and Linux Mint 21 are too old for the packaged libraries; build
from source there. CI installs the package on Ubuntu 24.04, and it has also been
installed and run on Debian testing (KDE Plasma, Wayland). Debian 13 and GNOME
installs have not yet been tested by the maintainer.

Run these in a normal terminal (`sudo` asks for your password, so they will not work
from a tool that has no terminal):

```bash
# 1. Trust the repository's signing key
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://mikeholp87.github.io/Luma-Launcher/apt/luma-launcher.asc | sudo tee /etc/apt/keyrings/luma-launcher.asc >/dev/null

# 2. Add the repository
echo "deb [signed-by=/etc/apt/keyrings/luma-launcher.asc] https://mikeholp87.github.io/Luma-Launcher/apt/ ./" | sudo tee /etc/apt/sources.list.d/luma-launcher.list

# 3. Install
sudo apt update && sudo apt install luma-launcher
```

Step 2 is easy to skip, and without it `apt` reports `Unable to locate package
luma-launcher`: a plain `sudo apt update` only helps once the repository is added.

**Check the key (optional).** The signing key's fingerprint is
`82A6 ED17 14A6 C92E FEFA  712A 1836 6AD3 BA2B FAAC`. Compare it with:

```bash
gpg --show-keys --with-fingerprint /etc/apt/keyrings/luma-launcher.asc
```

### After installing

Installing does not open anything by itself: Luma is a launcher that waits for you
to call it.

1. **Start it:** run `luma-launcher` or choose **Luma Launcher** in your application menu.
   It stays resident so later activations are instant.
2. **Bind a shortcut** (for example Alt+Space) to the command `luma-launcher --toggle`;
   see [Assign a global shortcut](#assign-a-global-shortcut).
3. **Optional:** turn on **Start at login** in Settings (Ctrl+, opens Settings).

### Update, check and remove

```bash
sudo apt update && sudo apt install --only-upgrade luma-launcher   # update
apt-cache policy luma-launcher                                     # installed vs available version
sudo apt remove luma-launcher                                      # remove (keeps your settings)
sudo rm /etc/apt/sources.list.d/luma-launcher.list /etc/apt/keyrings/luma-launcher.asc && sudo apt update   # forget the repository
```

Your preferences live in `~/.config/luma-launcher` and `~/.local/state/luma-launcher`,
and `apt remove` leaves them alone.

### Already installed from source or with `install.sh`?

Remove that copy first. `~/.local/bin` comes before `/usr/bin` on most systems, so the
old copy would keep running and hide the packaged one:

```bash
luma-launcher --quit
bash scripts/uninstall.sh          # keeps your settings
sudo apt install luma-launcher     # after adding the repository above
hash -r; type -a luma-launcher     # should list only /usr/bin/luma-launcher
```

A desktop shortcut that points at the old path (KDE stores it in
`~/.local/share/applications/`) must be changed to `luma-launcher --toggle` or
`/usr/bin/luma-launcher --toggle`. KDE keeps shortcut commands in memory, so log out
and back in afterwards.

## Features

- **Application search:** exact, prefix, substring, fuzzy subsequence, acronym,
  keyword, generic-name, and optional description matching.
- **System settings search:** like KRunner, typing "mouse" or "wifi" finds the
  matching KDE System Settings module (or GNOME Settings panel on GNOME) and
  opens that page directly. Settings pages rank after applications that match
  equally well and are not listed until you type.
- **Unicode support:** case folding and normalization, including composed and
  decomposed characters.
- **Native application metadata:** names, descriptions, themed icons, desktop
  actions, visibility rules, and desktop-ID precedence through GIO.
- **Keyboard navigation:** select results, launch, dismiss, pin favorites, remove
  history entries, and open settings without reaching for the mouse.
- **Personalized ranking:** favorites, most-used applications first (a running
  launch tally optionally combined with the desktop environment's own usage
  records), then recent launches, while preserving the priority of exact matches.
- **Resident activation:** show, hide, toggle, and refresh commands communicate
  with one application instance through the desktop session bus.
- **Event-driven discovery:** application-change notifications refresh the index
  without continuous directory scanning.
- **Appearance controls:** dark, light, and system theme modes; accent color,
  width, result count, font size, and compact layout settings.
- **Quick tools:** a bounded calculator (powers, percentages, functions, hex and
  binary) with clipboard copying, configurable search keywords, explicit
  HTTP/HTTPS URL opening, and declared desktop actions.
- **Local privacy:** no telemetry, advertising, downloaded application icons, or
  tracking of activity outside Luma. Launch history can be disabled or cleared.
  When "system app usage" is enabled, Luma reads the desktop environment's own
  usage tally read-only to rank applications — GNOME Shell's
  `application_state` or KDE's activity statistics — and never writes to it.

## Linux requirements

| Component | Requirement |
|---|---|
| Operating system | Linux. The apt package needs Debian 13 / Ubuntu 24.04 or newer (amd64); build from source elsewhere |
| Desktop | A graphical GTK4-compatible session with session D-Bus |
| Toolchain | Rust 1.92 or newer (the gtk-rs 0.11 MSRV) and Cargo |
| Native libraries | GTK4 runtime, GLib/GIO, a C linker, and pkg-config |
| Packaging tools | dpkg-dev and desktop-file-utils for Debian packaging |

The interface uses the maintained gtk-rs bindings (`gtk4`, `gio`, `glib` 0.22
and `gio-unix`), and the application crates forbid `unsafe` code.

## Build from source (Linux)

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

Run graphical commands from your normal desktop session, without sudo. Building
fetches crates from crates.io; once the registry cache is populated, `--offline`
also works. GTK widget tests need a display and skip themselves without one; run
them headless with `xvfb-run -a cargo test --locked`.
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

### Install with apt

See [Install (Debian, Ubuntu and derivatives)](#install-debian-ubuntu-and-derivatives)
near the top of this page.

### Install a single release (no repository, no updates)

Download the `.deb` and `SHA256SUMS` from the
[latest release](https://github.com/mikeholp87/Luma-Launcher/releases/latest), then:

```bash
sha256sum -c --ignore-missing SHA256SUMS
sudo apt install ./luma-launcher_*_amd64.deb
```

Releases are cut as described in [docs/releasing.md](docs/releasing.md).

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

Dependencies are generated from the executable's actual shared-library usage, so
the package needs at least the glibc and library versions of the machine that built
it. Build on the oldest distribution you want to support; the
[CI workflow](.github/workflows/check.yml) builds on Debian 13 and uploads the
`.deb` as an artifact, and a [container recipe](packaging/Containerfile) is supplied.

Package metadata (maintainer, homepage, recommended packages, description) lives in
[`debian/control`](debian/control). The package also ships a man page
([`packaging/luma-launcher.1`](packaging/luma-launcher.1)), a machine-readable
[`debian/copyright`](debian/copyright), `md5sums` for `debsums`, and the changelog.

To cut a release, bump `version` in `Cargo.toml` **and** add a matching entry at the
top of [`debian/changelog`](debian/changelog); `scripts/package.sh` refuses to build
when the two differ.

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
| Ctrl+K / Ctrl+J, Ctrl+P / Ctrl+N | Select the previous or next application |
| Ctrl+1 … Ctrl+9 | Launch the Nth visible result |
| Esc | Clear the search; press again to close |
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
| `= 2^10`, `= sqrt(2) * pi` | Powers, functions and constants |
| `= 200 + 15%` | Percentages work like a phone calculator: `230` |
| `= 255 in hex`, `= 0b1010 in dec` | Convert whole numbers between hex, binary and decimal |
| `g best linux utilities` | Search Google in the default browser |
| `ddg rust gtk4 tutorial` | Search DuckDuckGo in the default browser |
| `gh luma` | Any search keyword you add in Settings |
| `github.com/rust-lang` | Open a bare domain (`.com .org .net .io .dev .app .ai .edu .gov`) over HTTPS |
| `https://www.rust-lang.org/` | Offer to open the URL in the default browser |

The calculator supports `+ - * / ^` (powers bind tighter than signs, so `-2^2`
is `-4`), postfix `%`, parentheses, `0x`/`0b` literals, `pi`, `e`, and `sqrt`,
`abs`, `round`, `floor`, `ceil`, `ln`, `log` (base 10), `sin`, `cos` and `tan`
(radians). It is a bounded hand-written parser that rejects invalid or nonfinite
results and never evaluates executable code.

Search keywords are edited in Settings, one `keyword URL` pair per line, or in
`config.toml`. `%s` in the URL is replaced by the percent-encoded search terms,
and only `http` and `https` URLs are accepted. Defining the table replaces the
built-in `g` and `ddg` keywords:

```toml
[search_engines]
g = "https://www.google.com/search?q=%s"
gh = "https://github.com/search?q=%s"
w = "https://en.wikipedia.org/wiki/%s"
``` Arbitrary shell-command
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
system_usage = true
descriptions = true
fuzzy = true
clear_on_show = true
portal_shortcut = false
shortcut = "ALT+space"
```

Themes are `dark`, `light`, or `system`; `system` follows the desktop-wide colour
preference from the settings portal (KDE and GNOME), falling back to GNOME's
GSettings key. Width accepts 360–1200 pixels and applies the next time the palette
opens. Font size (12–32) scales the search box, result names, descriptions, footer
and icons together. Result count is 1–20. Accent colors use `#RRGGBB`. Settings
changes, including text fields, save automatically as you move on. Restart Luma after editing TOML externally.

History only records applications launched through Luma. Disabling history stops
new records; clear existing records separately in Settings. When "system app
usage" is on, applications already tallied by the desktop are ranked by that
combined usage, most used first and most recently used as a tiebreaker. The
source follows the session: GNOME Shell's `score` and `last-seen` in
`~/.local/share/gnome-shell/application_state`, or the application launches
in KDE's activity statistics (files and folders an app opened do not count)
in `~/.local/share/kactivitymanagerd/resources/database` (read through
`sqlite3 -readonly`; the `sqlite3` CLI must be installed, otherwise KDE usage is skipped). `luma-launcher --diagnostics` prints the source in use and whether `sqlite3` was found.
Ctrl+Delete removes an application from that combined ranking until it is
launched through Luma again. Writes use exclusive temporary files, atomic
replacement, and private state-file permissions. Corrupt files are preserved
rather than silently overwritten. Repair them and restart.

## Tests and verification

```bash
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
cargo test --locked --release --test glib_variant
bash scripts/test-install.sh
```

`bash scripts/test.sh` runs the same core checks (under Xvfb when it is installed).
Shell scripts are also checked with ShellCheck and shfmt.

The CI run is the authority on the current test count. The suite includes a
widget test that needs a display (it skips itself without one and runs under Xvfb in CI).
Coverage includes matching/ranking, Unicode, input limits, config corruption,
concurrent persistence, portal response parsing, keyboard-controller setup, theme CSS parsing/contrast,
calculator/URL safety, GIO launching, XDG precedence, and installer path quoting.
Application code passes strict Clippy, and `cargo audit --deny warnings` is clean.

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
| 100 | 9 µs | 9 µs |
| 1,000 | 56 µs | 102 µs |
| 10,000 | 747 µs | 783 µs |

These matching/sorting measurements are from a follow-up development run, not GTK
rendering or global-shortcut activation times. They do not establish a warm
activation, idle CPU, or resident-memory guarantee. Reproduce search measurements
with `cargo bench --bench search`; [methodology and limitations](docs/performance.md)
include the separate Hyperfine CLI measurements.

## Project layout

```text
src/          GTK interface, desktop integration, search, state, and quick tools
windows/      Windows PowerShell/WPF launcher, core tests, and usage guide
tests/        CLI, GIO, installation, and dependency regression tests
benches/      Synthetic search benchmark
assets/       Application icon
scripts/      Build checks, local installation/removal, autostart, and packaging
packaging/    Desktop entry and Debian Stable container recipe
debian/       Debian package metadata
docs/         Architecture, setup, security, audits, and test reports
```

Read the [architecture guide](docs/architecture.md) for lifecycle and data flow.
Future work is tracked in the verification report: real desktop integration tests, Stable package validation, richer settings, and
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
- **`E: Unable to locate package luma-launcher`:** the repository was not added; run
  steps 1 and 2 of [Install](#install-debian-ubuntu-and-derivatives), then `sudo apt update`.
- **`NO_PUBKEY`, "not signed" or "the key(s) … are ignored" during `apt update`:** the key
  file is missing or empty. Check `wc -c /etc/apt/keyrings/luma-launcher.asc` (about 409
  bytes) and repeat step 1.
- **`unmet dependencies … libc6 (>= 2.39)`:** the distribution is too old for the packaged
  build; see the supported releases above.
- **Installed, but nothing appears:** Luma only opens when called. Run `luma-launcher`,
  choose it in the application menu, or press your shortcut; bind one to
  `luma-launcher --toggle` if you have not.
- **The old version keeps running after an upgrade or reinstall:** a resident process owns
  the app's D-Bus name and answers commands. Run `luma-launcher --quit`, then start it
  again.

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

For a Debian package installation:

```bash
luma-launcher --quit
sudo apt remove luma-launcher
sudo rm /etc/apt/sources.list.d/luma-launcher.list /etc/apt/keyrings/luma-launcher.asc   # optional: forget the repository
```

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

Luma Launcher is available under the [MIT License](LICENSE).
