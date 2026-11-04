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

Unsafe code is confined to GTK ABI declarations and UI calls, all on GTK's main
thread. This is a material review surface; migrating to gtk-rs is recommended once
dependencies can be fetched. No unsafe thread transfers of widget pointers occur.

Validation performed: arithmetic/URL/config tests, GIO fixture launch, strict
Clippy, shell lint, desktop-entry validation, and Gitleaks (no secrets found).
The initial concurrent scan encountered disappearing build objects; rerun scans
after builds finish. Dependency vulnerability auditing is **not completed**:
cargo-audit is absent and network DNS is unavailable. The audit found and backported the upstream fix for [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) in vendored GLib. The unpatched optimized iterator test crashed with SIGSEGV; it now passes. See [backport provenance](../vendor/README.md). This targeted check is not a complete advisory scan.

Before production release: run an updated RustSec audit, migrate or independently
review GTK FFI, exercise portal denial/restart and session loss, inspect all
generated Debian dependencies, and complete desktop/UI tests in verification.md.
