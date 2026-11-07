# Audit and fixes

What this repo does: Luma is a resident GTK4 launcher that discovers applications
through GIO, ranks search results, and launches applications through desktop APIs.
It stores preferences locally and supports desktop-managed or portal shortcuts.
This audit assumes one desktop user, thousands of indexed entries, rapid typing,
repeated activation, malformed preferences, and failed filesystem or portal calls.

Reviewed all application Rust modules, FFI declarations, build/dependency setup,
tests, shell scripts, desktop metadata, Debian/container/CI configuration, and
project documentation. The vendored GLib review covers the specific upstream
security patch, not the entire upstream library.

## Must fix — corrected

> Historical record. Items 1 and the raw GTK ABI layer were later resolved by the
> gtk-rs migration (glib/gio 0.22, no vendored GLib, no `unsafe` in the application).

1. **Optimized GLib iterator crashes** (`vendor/glib/src/variant_iter.rs`).
   **What:** Luma depends on GLib 0.18.5.
   **Problem:** A string iterator passes an immutable pointer as a mutable C output
   argument. The new optimized regression test terminated with SIGSEGV.
   **Fix:** Vendor the cached crate and apply upstream's two-line mutability fix;
   the same optimized test now passes. Preserve upstream license and provenance.
   **If skipped:** The affected iterator remains unsound in release builds.
   Sources: [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429.html),
   [upstream fix](https://github.com/gtk-rs/gtk-rs-core/pull/1343).

2. **Successful portal sessions were rejected** (`src/portal.rs`).
   **What:** Shortcut registration parses CreateSession results.
   **Problem:** The protocol returns `session_handle` as a string, but Luma read
   an object-path variant. A valid response therefore failed registration.
   **Fix:** Read the string and validate its object-path syntax; cover both valid
   and invalid paths in a regression test.
   **If skipped:** Portal shortcuts cannot register.
   Source: [GlobalShortcuts API](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html).

3. **Shortcut failures left sessions and retry state behind** (`src/portal.rs`,
   `src/ui.rs`). **What:** Portal bindings have a session and signal subscriptions.
   **Problem:** Denial left subscriptions active and the UI permanently marked
   registration as active; an empty grant was also reported as success.
   **Fix:** Own subscriptions in a session guard, close failed sessions, accept
   only an actual toggle grant, reset pending state, use unique request tokens,
   and bound requests with timeout cleanup. Track session closure and portal
   owner loss; permit explicit re-registration. Validate activation tuples and
   forward activation tokens to GTK.
   **If skipped:** Retrying requires a restart and stale bindings accumulate.
   Pure protocol tests pass; live bus/service lifecycle testing remains blocked.

4. **A failed save could delete another pending file** (`src/state.rs`).
   **What:** Preferences use atomic replacement.
   **Problem:** Temporary names used only the PID. If exclusive creation failed,
   cleanup deleted the existing temporary file despite not owning it.
   **Fix:** Allocate a unique exclusive temporary file and clean up only a file
   created by that write. Regression tests preserve a pre-existing file and
   exercise eight concurrent writes, valid TOML, 0600 permissions, and cleanup.
   **If skipped:** A save can fail or destroy a pending write.

5. **Keyboard commands could reach the entry first** (`src/ui.rs`, `src/gtk.rs`).
   **What:** Launcher commands share keys with GTK text editing.
   **Problem:** A bubbling controller runs after the entry, which can consume
   Ctrl+J/K/D or interpret Ctrl+Enter as normal Enter.
   **Fix:** Use capture-phase handling; a headless GTK controller test verifies
   the actual propagation property.
   **If skipped:** Navigation, pinning, or desktop-action commands can perform
   text editing or launch the application instead.

6. **Tool results retained a hidden application selection** (`src/ui.rs`,
   `src/tools.rs`). **What:** Calculator and web results replace application rows.
   **Problem:** The application index results remained selected behind those rows,
   allowing pin/forget/actions to target an unrelated application.
   **Fix:** Classify tool input consistently, clear application results for tool
   views, and reject application selection for those queries. Add classification
   tests including leading whitespace and invalid calculations.
   **If skipped:** Application state can change while a tool result is displayed.

7. **Desktop entries relied on the session PATH** (`scripts/install.sh`,
   `scripts/autostart.sh`, `src/state.rs`). **What:** Desktop launch/autostart must
   locate the installed binary. **Problem:** Entries used a bare executable name;
   GUI autostart could also start a different installed version.
   **Fix:** Store quoted absolute executable paths, escape desktop-entry strings,
   and write entries atomically. GIO tests actually launch a harmless fixture from
   paths containing spaces, quotes, and backslashes with an unrelated PATH.
   **If skipped:** Installation appears successful but desktop launching can fail.
   Percent, equals, and control characters in executable paths are explicitly
   rejected before installation because of desktop-entry/GIO parsing limitations.

8. **Uninstall accepted unintended destructive arguments** (`scripts/uninstall.sh`).
   **What:** `--purge` removes Luma's preferences.
   **Problem:** Extra arguments were ignored, and relative XDG paths disagreed
   with Rust's directory selection.
   **Fix:** Reject unknown/extra arguments before writes, reject relative binary
   destinations, and use default homes for relative XDG values. The installer
   test confirms rejected arguments preserve preferences and unrelated files.
   **If skipped:** A malformed invocation can purge preferences unexpectedly or
   target a different directory than the running application uses.

## Should fix — corrected

9. **Unicode search rejected valid-length input** (`src/search.rs`).
   **What:** GTK permits 512 input characters. **Problem:** Search counted UTF-8
   bytes, so a 200-character CJK exact name returned no match.
   **Fix:** Enforce the limit in characters before normalization; reject NUL input.
   The previously failing CJK test passes.
   **If skipped:** Non-ASCII names have a much smaller effective search limit.

10. **Informational CLI flags bypassed validation** (`src/main.rs`).
    **What:** Version and diagnostics are display-independent commands.
    **Problem:** Combining them with invalid/conflicting flags silently succeeded.
    **Fix:** Validate arguments before handling informational commands. Tests check
    exit status 2 and independently confirm GIO forwards ordinary command flags.
    **If skipped:** Invalid invocations appear successful.

11. **Settings failures were displayed in the hidden palette** (`src/ui.rs`).
    **What:** Settings actions report save and shortcut errors.
    **Problem:** Messages updated only the main launcher footer, often hidden.
    **Fix:** Add a settings status label and keep it synchronized while the window
    exists. Also report shortcut preference-save failures instead of discarding them.
    **If skipped:** Users cannot see why settings or registration failed.
    This change is source-reviewed; visual verification remains unavailable.

12. **Input/file limits were applied inconsistently** (`src/tools.rs`,
    `src/state.rs`). **What:** Arithmetic and configuration accept bounded input.
    **Problem:** Arithmetic rejected internal tabs/newlines; file limits checked
    metadata before an unbounded read; shortcut strings had no basic validation.
    **Fix:** Accept ASCII whitespace in arithmetic, bound the actual file read,
    and reject empty, overlong, or control-containing shortcut settings.
    **If skipped:** Equivalent arithmetic fails and file/config bounds are unreliable.
    Tests cover whitespace, oversized/corrupt files, and invalid shortcut strings.

## Validation and remaining limits

Run `bash scripts/test.sh` for formatting, strict application Clippy, all tests,
release compilation, the optimized GLib regression, and isolated install/removal.
ShellCheck, shfmt, desktop-entry validation, secret scanning, and Debian package
checks are also part of the audit handoff. See verification.md for final counts.

Initial audit results: 28 tests passed, one graphical test ignored; release compilation,
the optimized GLib regression, ShellCheck, shfmt, formatting, installer tests,
desktop validation, and Gitleaks passed. Debian extraction verified an exact
executable match and included upstream licensing. The installed executable at
`/home/mikeh/.local/bin/luma-launcher` also exactly matched the audited release
when checked. Diagnostics reported 136 installed applications and an inaccessible
session bus. Restart a running Luma instance from the real desktop session to
load the updated executable; process reload could not be performed here.

The vendored upstream code emits 34 existing Rust style/lifetime warnings with
Rust 1.96. Application code passes `-D warnings`; these dependency warnings were
left unchanged to keep the security backport limited to the upstream fix.
The complete RustSec dependency audit is unavailable: cargo-audit is absent and
registry DNS is blocked. One confirmed advisory was investigated and fixed;
absence of other advisories is not asserted.

Verdict: confirmed findings above are corrected; production acceptance still
requires a real desktop and complete dependency audit.

Not checked: live KDE/GNOME shortcut registration, compositor focus, suspend/resume,
HiDPI/multiple monitors, visual rendering, Stable container/package installation,
and independent review of the complete third-party/native libraries. One graphical
test remains explicitly ignored. No “bug-free” or production-ready claim is made.

## Follow-up on 2026-11-04

After subsequent launcher and UI changes, `cargo fmt --check`, strict Clippy,
`cargo test --locked` (41 passed, one ignored), and release compilation passed.
Gitleaks scanned the repository after compilation and found no leaks. ShellCheck
and shfmt passed. Search now precomputes normalized descriptions, and child output
logs rotate at 5 MiB between launches, retaining one prior log. A single long-lived
or unusually verbose child can still grow its current log beyond that threshold.
Dependency advisory scanning and live desktop testing remain outstanding.
