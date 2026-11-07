# Performance measurements

Measured in this restricted Debian forky/sid environment, x86-64 Intel Core 7 150U,
12 logical CPUs, release optimization with thin LTO. These results do not establish
desktop responsiveness or compositor latency.

`cargo bench --locked --bench search`: 100 repetitions per dataset, synthetic
“Application N” entries, query `app 99`, result limit 8. Microsecond resolution.
Latest run after precomputing normalized descriptions:

| Entries | p50 | p95 |
|---|---:|---:|
| 100 | 9 µs | 9 µs |
| 1,000 | 56 µs | 102 µs |
| 10,000 | 747 µs | 783 µs |

The earlier run reported p95 118/840/5,537 µs respectively. Benchmark runs can
vary; these numbers are observations, not a controlled before/after claim. The
benchmark includes matching and sorting, not GTK rendering.

Hyperfine, two warmups and 20 repetitions per command:

| Command | Mean ± standard deviation | Reported memory |
|---|---:|---:|
| `--version` | 24.2 ± 9.7 ms | 23.1 MiB |
| `--diagnostics` | 53.8 ± 14.1 ms | 29.9 MiB |

Diagnostics discovered 135 applications. These are fresh CLI processes with warm
filesystem caches, not cold-cache GTK startup or warm resident activation. The
memory values belong to those short-lived commands, not hidden/visible UI RSS.

Unmeasured: activation p50/p95, first frame, focus readiness, result rendering,
resident RSS, idle CPU, refresh latency, and launch initiation timing. Display and
session-bus restrictions prevent collecting them here. No target is claimed met
except synthetic search capacity and its measured latency.

On a real session, start `--background`, instrument command receipt and GTK frame
presentation, and measure at least 100 activations. Sample `/proc/PID/status` while
hidden/visible and CPU ticks over 60 seconds idle. Keep compositor presentation and
launched-application startup separate. Repeat with both desktop environments and
Wayland/X11. Use `scripts/benchmark.sh` for the reproducible noninteractive subset.

## gtk-rs migration check

Head-to-head, same machine, `hyperfine -N --warmup 5 --runs 60`, release builds
before (GTK ABI shim, glib 0.18) and after (gtk4 0.11, glib/gio 0.22):

| Command | Before | After |
|---|---:|---:|
| `--version` | 23.0 ± 4.9 ms | 20.9 ± 5.0 ms |
| `--diagnostics` | 53.6 ± 9.7 ms | 61.0 ± 16.4 ms (ranges overlap; minimums 41.6 vs 41.2 ms) |

Binary size 1,350,960 → 1,344,176 bytes; `ldd` lists the same 114 lines. Resident
memory under Broadway stayed flat (about 68 MB) across 100 palette toggles and 40
Settings open/close cycles, so no window or closure leaks were observed. This is
an isolated headless run, not compositor latency.
