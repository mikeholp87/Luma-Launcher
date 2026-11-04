# Performance measurements

Measured in this restricted Debian forky/sid environment, x86-64 Intel Core 7 150U,
12 logical CPUs, release optimization with thin LTO. These results do not establish
desktop responsiveness or compositor latency.

`cargo bench --offline --bench search`: 100 repetitions per dataset, synthetic
“Application N” entries, query `app 99`, result limit 8. Microsecond resolution.

| Entries | p50 | p95 |
|---|---:|---:|
| 100 | 91 µs | 118 µs |
| 1,000 | 510 µs | 840 µs |
| 10,000 | 4,846 µs | 5,537 µs |

The earlier run reported p95 75/850/5,303 µs respectively. This variation is not an
optimization claim. The benchmark includes matching and sorting, not GTK rendering.

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
