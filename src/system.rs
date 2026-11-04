//! Read-only usage data harvested from the desktop environment.
//!
//! GNOME Shell keeps its own running tally of application usage in
//! `$XDG_DATA_HOME/gnome-shell/application_state`. Entries carry a `score`
//! (roughly minutes of focused use) and `last-seen`, a Unix timestamp of the
//! most recent focus. Nothing here writes to that file.

use crate::state::Usage;
use std::{collections::BTreeMap, fs, io, io::Read, path::PathBuf};

pub fn gnome_state_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))
        })?;
    Some(base.join("gnome-shell").join("application_state"))
}

fn attribute<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let start = format!("{name}=\"");
    let at = line.find(&start)? + start.len();
    let rest = &line[at..];
    Some(&rest[..rest.find('"')?])
}

fn tally(raw: &str) -> Option<u64> {
    let value: f64 = raw.parse().ok()?;
    if value.is_finite() && value >= 0. {
        Some(value.floor() as u64)
    } else {
        None
    }
}

fn parse(contents: &str) -> BTreeMap<String, Usage> {
    let mut usage = BTreeMap::new();
    for line in contents.lines() {
        if !line.trim_start().starts_with("<application") {
            continue;
        }
        let Some(id) = attribute(line, "id") else {
            continue;
        };
        let id = id.trim();
        if !id.ends_with(".desktop") || id.chars().any(char::is_whitespace) {
            continue;
        }
        let Some(raw_count) = attribute(line, "score") else {
            continue;
        };
        let Some(count) = tally(raw_count) else {
            continue;
        };
        let Some(raw_last) = attribute(line, "last-seen") else {
            continue;
        };
        let Some(last) = tally(raw_last) else {
            continue;
        };
        usage.insert(id.to_string(), Usage { count, last });
    }
    usage
}

/// Desktop usage recorded by GNOME Shell, keyed by desktop file ID.
pub fn gnome_usage() -> BTreeMap<String, Usage> {
    let Some(path) = gnome_state_path() else {
        return BTreeMap::new();
    };
    let read = || -> io::Result<String> {
        let file = fs::File::open(&path)?;
        let mut contents = String::new();
        // Matches the settings limit; refuse to read a pathological file.
        file.take(4 * 1024 * 1024 + 1)
            .read_to_string(&mut contents)?;
        if contents.len() > 4 * 1024 * 1024 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
        }
        Ok(contents)
    };
    read().map(|contents| parse(&contents)).unwrap_or_default()
}

/// Combine the launcher's own launch tally with desktop usage. Both sources
/// contribute one running tally per application; recent-use timestamps take
/// the later of the two. Entries the user recorded as ignored are skipped.
pub fn merge(
    launcher: &BTreeMap<String, Usage>,
    system: &BTreeMap<String, Usage>,
    ignored: &std::collections::BTreeSet<String>,
) -> BTreeMap<String, Usage> {
    let mut merged = launcher.clone();
    for (id, entry) in system {
        if ignored.contains(id) {
            continue;
        }
        let usage = merged.entry(id.clone()).or_default();
        usage.count = usage.count.saturating_add(entry.count);
        usage.last = usage.last.max(entry.last);
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_usage() -> BTreeMap<String, Usage> {
        parse(
            r#"<?xml version="1.0"?>
<application-state>
  <context id="">
    <application id="org.gnome.Yelp.desktop" score="0" last-seen="1793771310"/>
    <application id="com.mitchellh.ghostty.desktop" score="89.5" last-seen="1793774289"/>
    <application id="not installed" score="3" last-seen="1"/>
    <application id="partial.desktop" score="junk" last-seen="2"/>
  </context>
</application-state>"#,
        )
    }

    #[test]
    fn parses_gnome_application_state() {
        let usage = sample_usage();
        assert_eq!(
            usage.get("com.mitchellh.ghostty.desktop"),
            Some(&Usage {
                count: 89,
                last: 1793774289
            })
        );
        assert_eq!(
            usage.get("org.gnome.Yelp.desktop"),
            Some(&Usage {
                count: 0,
                last: 1793771310
            })
        );
        assert_eq!(usage.len(), 2);
    }

    #[test]
    fn incomplete_lines_are_skipped() {
        assert!(parse("not xml at all").is_empty());
        assert!(parse("").is_empty());
    }

    #[test]
    fn merge_combines_tallies_and_latest_use() {
        let launcher = BTreeMap::from([
            (
                "a.desktop".to_string(),
                Usage {
                    count: 3,
                    last: 500,
                },
            ),
            (
                "b.desktop".to_string(),
                Usage {
                    count: 1,
                    last: 100,
                },
            ),
        ]);
        let system = BTreeMap::from([
            (
                "a.desktop".to_string(),
                Usage {
                    count: 7,
                    last: 300,
                },
            ),
            (
                "b.desktop".to_string(),
                Usage {
                    count: 2,
                    last: 900,
                },
            ),
            (
                "c.desktop".to_string(),
                Usage {
                    count: 5,
                    last: 700,
                },
            ),
        ]);
        let ignored = ["c.desktop".to_string()].into_iter().collect();
        let merged = merge(&launcher, &system, &ignored);
        assert_eq!(
            merged.get("a.desktop"),
            Some(&Usage {
                count: 10,
                last: 500
            })
        );
        assert_eq!(
            merged.get("b.desktop"),
            Some(&Usage {
                count: 3,
                last: 900
            })
        );
        assert_eq!(merged.get("c.desktop"), None);
    }
}
