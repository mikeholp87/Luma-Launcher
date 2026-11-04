# Architecture

One GApplication owns the launcher and settings windows. Its application ID is
`io.github.luma_launcher.LumaLauncher`. GApplication forwards command-line requests
over the session bus. A hold guard keeps the resident application alive when hidden.
No lockfiles or privileged services are installed. Startup refuses to continue if
the session bus cannot be reached, avoiding an uncoordinated duplicate process.

GIO enumerates desktop applications, applying XDG precedence and visibility. A
worker thread creates owned metadata snapshots. AppInfoMonitor drives refreshes;
GTK consumes completed snapshots on its main context. Launching uses DesktopAppInfo
and a GDK launch context; Exec fields are never interpreted by Luma.

Search uses precomputed casefolded, normalized names, metadata, and acronyms. Match
tiers keep exact names ahead of personalization. Ties use favorites, recency,
bounded frequency, normalized name, then desktop ID. The initial matcher is a
Unicode subsequence matcher, not nucleo. Search and bounded row construction
currently run synchronously on the GTK thread; measurements are in performance.md.

GTK4 is called through the isolated `src/gtk.rs` ABI declarations and UI functions.
GObject signals use GLib's reference-counted wrappers. All widget calls occur on
the main thread. This introduces unsafe code and needs interactive validation and
eventual migration to gtk-rs when dependencies can be fetched.

Portal setup uses asynchronous GIO D-Bus calls, subscribing to Request.Response
before issuing each request. Granted bindings toggle the resident application.
The fallback opens desktop shortcut settings. Compositors control positioning and
may restrict focus without an activation token; Luma does not bypass that policy.

Configuration and state are TOML files under XDG config/state homes. State writes
use exclusive temporary files, fsync, atomic rename, and directory sync. Corrupt
files are preserved and writes to them are blocked for that session. Defaults
apply in memory. History is capped at 1,000 application IDs.
