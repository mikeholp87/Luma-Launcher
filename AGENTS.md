# Repository Guidelines

## Project Structure & Module Organization

This `luma-launcher` workspace currently contains no application source, assets, package manifests, or test configuration. The hidden `.agents/`, `.codex/`, and `.aws/` directories are environment metadata; do not use them for application code or commit credentials.

When adding the initial implementation, group source, tests, and assets clearly (for example, `src/`, `tests/`, and `assets/`). Update this guide with the actual layout once established.

## Build, Test, and Development Commands

No build, test, or local development commands are configured yet. Do not assume `npm test`, `npm run build`, or another toolchain works. When introducing a build system, document its prerequisites and exact install, development, build, and test commands here.

For repository exploration, use `rg --files --hidden` to list files and `rg "pattern" <path>` to search content. Prefix shell commands with `rtk`; use `rtk proxy <command>` for commands that need unfiltered output.

## Coding Style & Naming Conventions

No language, indentation standard, formatter, or linter has been established. Follow the surrounding code when extending the project. When adding its first source files, choose consistent language-appropriate formatting and descriptive names. Avoid introducing dependencies or abstractions without a concrete need.

Lint any shell scripts you add or edit with `shellcheck <file>` and check formatting with `shfmt -d <file>`.

## Testing Guidelines

No testing framework or coverage threshold is configured. Add focused tests for nontrivial behavior and regression tests for bug fixes. Use descriptive test names and document the chosen framework, file naming convention, and execution command when tests are introduced.

## Commit & Pull Request Guidelines

Git history is unavailable in this workspace, so no existing commit convention can be verified. Use concise, imperative commit subjects. Pull requests should explain the change, link relevant issues, report validation performed, and include screenshots for visible interface changes.

## Security & Configuration

Never commit secrets. Before pushing or after changing configuration or environment files, run `gitleaks git --redact --no-banner` in a valid Git checkout. Scan untracked files with `gitleaks dir <path> --redact --no-banner`.
