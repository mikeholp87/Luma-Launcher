# Verification and acceptance status

This is an initial implemented release, **not production verified**. No graphical
desktop combination was exercised successfully. Preserve that distinction when
sharing binaries.

## Executed

- Release compilation succeeds using cached dependencies; executable in
  `target/release/luma-launcher`.
- Initial baseline: eleven automated tests passed: matching, ranking, Unicode casefold/composition,
  query limits, configuration validation/corruption, persistence, calculator,
  URL safety, desktop metadata/visibility, GIO launch, and XDG precedence/dedup.
- Formatting, strict Clippy across targets/features, ShellCheck, shfmt, and desktop
  entry validation pass.
- Diagnostics execute and discover 135 host applications.
- GIO integration launches `/usr/bin/true`; no arbitrary user's app was launched.
- Synthetic benchmarks exercise 100, 1,000, and 10,000 entries.
- Local installation/removal tested in temporary XDG directories: installed
  executable runs, autostart entry is created, uninstall preserves preferences,
  and explicit purge removes them.
- Host Debian package builds; its control metadata and contents can be inspected
  with dpkg-deb. Package installation by the package manager remains untested.
- Extracted the final package, verified its executable exactly matches the release
  build, ran its `--version`, and validated its desktop entry and license payload.
- Final graphical startup correctly refuses to proceed when session-bus access
  is denied. The earlier build also demonstrated the display-access restriction.
- Follow-up checks: the full test suite (see CI for the current count), formatting, strict Clippy, and release build
  pass again. `bash scripts/test-install.sh` passes installation/removal checks
  including spaces in paths, icons, autostart enable/disable, preservation, purge,
  and preservation of unrelated files. ShellCheck and shfmt also pass.
- Actual user-local installation was attempted but blocked: `/home/mikeh/.local/bin`
  is read-only in the sandbox. A private D-Bus test also failed because socket
  creation is denied. Installation must run from the user's desktop terminal.

## Audit follow-up

The full audit and fix record is [audit.md](audit.md). The current suite contains
41 passing tests and one explicitly ignored graphical test. The optimized GLib
iterator regression also passes after reproducing a SIGSEGV before the upstream
fix was backported. Strict application Clippy passes; the vendored dependency
retains 34 upstream style/lifetime warnings. Installer tests now exercise actual
GIO launches of a harmless fixture with an unrelated PATH, including executable
paths containing spaces, quotes, and backslashes. Secret and package checks are
recorded in the audit handoff. Full dependency auditing and live desktop coverage
remain incomplete.

## Implemented but unverified interactively

Palette rendering, initial focus, keyboard/mouse selection, Enter/Escape, icons,
single-instance command forwarding, portal shortcut consent/activation, desktop
actions, light/dark appearance, settings controls, and resident idle behavior.
GUI launch returned “Failed to open display”; session D-Bus access is denied.

| Desktop | Wayland | X11 |
|---|---|---|
| GNOME | Not exercised; sandbox blocks access | Not available |
| KDE Plasma | Not available | Not available |

## Remaining work before production acceptance

- Exercise each matrix cell: real Alt+Space registration/conflict, repeated toggle,
  typing, launch, actions, missing icons, theme changes, 2× scaling, multiple
  monitors, suspend/resume, login autostart, session loss, and removal during search.
- Verify granted portal bindings restore correctly and portal service-loss cleanup
  works on a real bus. Session cleanup, failure retry, empty-grant rejection, and
  protocol validation are implemented, with pure protocol regression tests. A
  private-bus mock integration test remains blocked by denied socket creation.
- Build/run the Debian Stable container or CI job and install its package on
  Stable and Testing. Container/Docker access is unavailable locally; workflow is
  supplied but has not run here.
- Complete performance measurements listed in performance.md and dependency audit.
- Add worker-based latest-query search and result-widget reuse if profiling shows
  the synchronous implementation misses responsiveness targets.
- Expand settings: hidden-app controls, separate favorites/recent display/ranking
  options, configurable web providers, transparency, and full keyboard preferences.
- Improve GNOME/KDE system-theme parity. Current system mode follows GNOME's
  color-scheme setting when present; KDE theme integration needs validation.
- No animated transitions are implemented; rendering is static and therefore
  reduced-motion safe. No arbitrary command mode, custom compositor positioning,
  forced focus, or custom icon cache was added.

Useful next priorities: obtain an accessible GUI test session; migrate the offline
GTK ABI layer to gtk-rs; add portal lifecycle tests; verify Stable packaging. Do
not treat these remaining acceptance checks as passed based on source inspection.
