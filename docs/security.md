# Security review

Luma runs as the desktop user. It installs no privileged service, keyboard hook,
or network listener. No telemetry, icon downloads, or unrelated activity tracking
is implemented. Web search and URL opening intentionally invoke the user's browser.

Application execution goes through GIO DesktopAppInfo. Luma never interpolates
application names or search text into a shell. Desktop actions execute only action
IDs declared by the selected installed desktop entry. Installed desktop entries
and their Exec fields are trusted user/system application configuration, not a
sandbox boundary. Desktop settings commands are fixed executable/argument pairs.

Ordinary search cannot execute commands. HTTP/HTTPS URLs require a host and cannot
contain credentials. Search terms are encoded as URL query parameters. Arithmetic
uses a bounded parser, no code evaluator; invalid/nonfinite results and division by
zero fail safely. Input is limited to 512 characters; arithmetic to 256 ASCII bytes
and nesting depth 24. No operations trigger merely by typing.

History records only launches through Luma. Disabling history stops new records;
existing records require an explicit clear. TOML state is written with mode 0600,
exclusive temporary creation, fsync and atomic rename. Corrupt files remain intact
and are not overwritten by settings. Files over 4 MiB are rejected; history has a
1,000-ID limit. Favorites remain separate from usage counts.

Global shortcuts use explicit portal consent or desktop-managed configuration.
Existing desktop shortcut assignments are never rewritten. Portal requests are
asynchronous; cancellation/failure produces a status message. The backend has not
been live-tested. Bus loss/restart and portal restart recovery need desktop testing.

The application crates forbid `unsafe` code (`#![forbid(unsafe_code)]`); all
unsafe FFI lives in the maintained gtk-rs bindings. Widgets are only touched on
GTK's main thread, and worker threads exchange plain owned data.

Validation performed: arithmetic/URL/config tests, GIO fixture launch, strict
Clippy, shell lint, desktop-entry validation, and Gitleaks (no secrets found).
Dependency auditing: `cargo audit --deny warnings` runs in CI. The earlier GLib 0.18
iterator crash ([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html))
and the unmaintained `proc-macro-error` dependency are gone with the move to
glib/gio 0.22; the vendored patch was removed.

Before production release: run an updated RustSec audit, migrate or independently
review GTK FFI, exercise portal denial/restart and session loss, inspect all
generated Debian dependencies, and complete desktop/UI tests in verification.md.
