# Linux feature comparison

Compared against GitHub `main` at
[`4fd7c9fe9dcba6853a5718451e22f064ef624136`](https://github.com/mikeholp87/Luma-Launcher/commit/4fd7c9fe9dcba6853a5718451e22f064ef624136),
reviewed November 8, 2026. Reviewed upstream README and `src/ui.rs`, `search.rs`,
`tools.rs`, `state.rs`, `main.rs`, and `launch.rs`. Existing Windows settings
and personalization are preserved.

| Linux feature/update | Windows implementation |
|---|---|
| Exact/prefix/word-start/substring tiers | Ported |
| Acronyms before fuzzy matching | Ported; works with fuzzy disabled |
| Tighter fuzzy matches first | Ported |
| Keywords/categories and optional description search | Shortcut descriptions, folder categories, executable names |
| Unicode full folding and normalization | Unicode 17 default folding and .NET compatibility composition |
| Favorites, logarithmic frequency buckets, then recency | Ported with Luma-local usage |
| Bounded history (1,000 entries) | Ported |
| Ctrl+J/K and Ctrl+N/P navigation | Ported |
| Ctrl+1–9 quick launch | Number row and numpad; application results only |
| Escape clears before hiding | Ported |
| Ctrl+comma/plus Settings | Ported including numpad plus |
| Ctrl+Enter declared desktop actions | Windows shell verbs and shortcut location |
| Bare domains over HTTPS | Ported with the same known TLDs |
| Clean calculator formatting | Ten fractional digits, scientific notation for large numbers |
| Bounded unary arithmetic | Parser with no executable-code evaluation |
| Automatic settings saves | Focus loss/Enter and immediate checkbox/theme saves |
| Save failure handling | Edits retained; toggles and shortcut bindings restored |
| Destructive personalization confirmation | Second click, persist before applying |
| Live system theme | Windows preference notifications and app theme registry value |
| Font scaling throughout palette | Search, names, descriptions, footer, icons |
| Compact layout, width, accent, result limit | Supported; content-sized scrolling palette |
| Hide on focus loss/successful launch | Supported with modal dialog handling |
| Event-driven app discovery | Background metadata indexing and debounced Start Menu watchers |
| Resident command-line controls | Current-user/session named pipe and mutex |
| Show/hide/toggle/settings/refresh/background/quit | Ported |
| Version and diagnostics | Windows runtime, executable, settings path, count and usage source |
| Atomic bounded persistence and corruption protection | Native JSON settings |
| Login startup | Current-user Windows Run value |
| Native icons | Windows shell shortcut icons with a generic fallback |
| GNOME/KDE external usage and ignore lists | Linux-only; Windows counts Luma launches |
| XDG shortcut portal and desktop shortcut settings | Windows uses RegisterHotKey and capture fields |
| GIO desktop IDs, visibility and XDG precedence | Windows Start Menu shortcuts |
| Application stdout/stderr log redirection | Linux-only; Windows retains ShellExecute shortcut/verb semantics |
| GTK migration, dependencies, installers and Linux CI | Linux-only; Windows uses WPF/.NET |

Unicode data comes from the Unicode Consortium's
[`CaseFolding.txt`](https://github.com/unicode-org/unicodetools/blob/main/unicodetools/data/ucd/17.0.0/CaseFolding.txt),
blob `a0b0f07fd64598e698fc7da3e55ca72077dd6071`, under
[`native/Unicode/LICENSE`](native/Unicode/LICENSE).
