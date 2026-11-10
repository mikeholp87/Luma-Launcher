# Releasing

A release is a version tag. CI (`.github/workflows/release.yml`) builds the `.deb`
on Ubuntu 24.04, runs the tests, installs the package on the runner, verifies and
removes it, then publishes a GitHub Release with the `.deb` and `SHA256SUMS`.

## Cut a release

1. Bump `version` in `Cargo.toml` (and let `Cargo.lock` follow: `cargo build`).
2. Add a new entry at the **top** of `debian/changelog` with the same version:

   ```text
   luma-launcher (0.2.0) unstable; urgency=medium

     * What changed, one bullet per user-visible change.

    -- Your Name <you@example.com>  Tue, 10 Nov 2026 09:00:00 +0700
   ```

   `date -R` prints a correctly formatted date. `scripts/package.sh` refuses to
   build when this version differs from `Cargo.toml`.
3. Check locally: `cargo build --release && bash scripts/package.sh`.
4. Merge to `main`, then tag and push:

   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

   The workflow fails early if the tag is not `v` plus the `Cargo.toml` version.

## Dry run

Opening a pull request that changes the workflow, `scripts/package.sh`, `debian/` or
`packaging/` runs the whole build (tests, package, install check) without
publishing. **Run workflow** on the Actions tab does the same from a branch.

## What users get

`sudo apt install ./luma-launcher_<version>_amd64.deb` from the Release page, on
Debian 13 or Ubuntu 24.04 and newer (amd64). The package's library requirements come
from the build machine, so supporting an older distribution means building on it.

Automatic updates need an apt repository, which is a separate step (publish the
`.deb` into a signed flat repository on GitHub Pages).
