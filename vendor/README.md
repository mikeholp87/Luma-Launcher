# GLib security backport

`glib/` is the published MIT-licensed glib 0.18.5 crate copied from the Cargo
registry cache. Its upstream license is preserved. Registry cache metadata files
were omitted because this directory is a patched path dependency.

The only source changes are in `src/variant_iter.rs`: make the output pointer
mutable and pass `&mut p`, following upstream commit
`b5a4071` in https://github.com/gtk-rs/gtk-rs-core/pull/1343.
This fixes https://rustsec.org/advisories/RUSTSEC-2024-0429.html.

The unpatched optimized regression test in `tests/glib_variant.rs` terminated with
SIGSEGV. Run `cargo test --release --test glib_variant` to verify this backport.

This is a temporary offline measure. Replace the patch with coordinated maintained
GIO/GLib bindings (GLib >=0.20) when registry access is restored. Version-based
auditors may still flag 0.18.5; retain the advisory record and source diff rather
than silently ignoring the warning. No other dependency is represented as audited.
