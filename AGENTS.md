# Repository Guidelines

## Project Structure & Module Organization

The Rust/GTK Linux launcher lives in `src/`, with tests in `tests/`, assets in `assets/`, and Linux packaging in `scripts/`, `packaging/`, and `debian/`. The Windows PowerShell/WPF preview lives in `windows/`. The hidden `.agents/`, `.codex/`, and `.aws/` directories are environment metadata; do not use them for application code or commit credentials.

Keep platform-specific implementations and their documentation together.

## Build, Test, and Development Commands

Linux builds require Rust, pkg-config, GLib/GIO and GTK4 development libraries. Rust 1.92+ is required. Use `cargo build --locked`, `cargo test --locked`, and `cargo build --locked --release`; the single GTK widget test needs a display and skips itself without one (run `xvfb-run -a cargo test --locked` for it). See README.md for prerequisites and further checks. The Windows WPF application requires the .NET 10 SDK; build with `dotnet build windows/native/LumaLauncher.csproj -c Release` or publish with `dotnet publish windows/native/LumaLauncher.csproj -c Release -o dist/LumaLauncher`. Run `dist/LumaLauncher/LumaLauncher.exe` with the .NET 10 Windows Desktop Runtime installed.

For repository exploration, use `rg --files --hidden` to list files and `rg "pattern" <path>` to search content. Prefix shell commands with `rtk`; use `rtk proxy <command>` for commands that need unfiltered output.

## Coding Style & Naming Conventions

No language, indentation standard, formatter, or linter has been established. Follow the surrounding code when extending the project. When adding its first source files, choose consistent language-appropriate formatting and descriptive names. Avoid introducing dependencies or abstractions without a concrete need.

Lint any shell scripts you add or edit with `shellcheck <file>` and check formatting with `shfmt -d <file>`.

## Testing Guidelines

Rust unit tests are colocated with source and integration tests live in `tests/`; run `cargo test --locked`. Run the compiled Windows core checks with `dotnet windows/native/bin/Release/net10.0-windows/LumaLauncher.dll --self-test` after building. The older PowerShell preview tests are `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\windows\Test-Core.ps1`. Add focused tests for nontrivial behavior and regression tests for bug fixes.

## Commit & Pull Request Guidelines

Git history is unavailable in this workspace, so no existing commit convention can be verified. Use concise, imperative commit subjects. Pull requests should explain the change, link relevant issues, report validation performed, and include screenshots for visible interface changes.

## Security & Configuration

Never commit secrets. Before pushing or after changing configuration or environment files, run `gitleaks git --redact --no-banner` in a valid Git checkout. Scan untracked files with `gitleaks dir <path> --redact --no-banner`.
