//! Read-only usage data harvested from the desktop environment.
//!
//! Two records are recognized and the one updated most recently wins, so a
//! live session trumps a stale one:
//!
//! - GNOME Shell keeps a running tally of application usage in
//!   `$XDG_DATA_HOME/gnome-shell/application_state`. Entries carry a `score`
//!   (roughly minutes of focused use) and `last-seen`, a Unix timestamp of
//!   the most recent focus.
//! - KDE's activity manager tallies resource use in
//!   `$XDG_DATA_HOME/kactivitymanagerd/resources/database` (SQLite), read
//!   here through `sqlite3 -readonly`.
//!
//! Nothing here writes to either file.

use crate::state::Usage;
use std::{
    collections::BTreeMap,
    fs, io,
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

fn xdg_data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
}

pub fn gnome_state_path() -> Option<PathBuf> {
    Some(
        xdg_data_dir()?
            .join("gnome-shell")
            .join("application_state"),
    )
}

pub fn kde_state_path() -> Option<PathBuf> {
    Some(
        xdg_data_dir()?
            .join("kactivitymanagerd")
            .join("resources/database"),
    )
}

/// Which per-desktop usage record applies to this session.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Backend {
    Gnome,
    Kde,
}
impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Gnome => "GNOME Shell application state",
            Self::Kde => "KDE activity statistics",
        }
    }
}

pub fn backend() -> Option<(Backend, PathBuf)> {
    // Sessions can outlive their environment: a stale GNOME record around an
    // active KDE desktop (or the reverse) must not win. The record updated
    // most recently represents current usage; only when neither exists on
    // disk does the session environment pick the initial backend.
    let (gnome, kde) = (gnome_state_path()?, kde_state_path()?);
    let (backend, path) = match (freshness(&gnome), freshness(&kde)) {
        (Some(g), Some(k)) if k > g => (Backend::Kde, kde),
        (Some(_), Some(_)) => (Backend::Gnome, gnome),
        (None, Some(_)) => (Backend::Kde, kde),
        (Some(_), None) => (Backend::Gnome, gnome),
        (None, None) => {
            let desktop = std::env::var("XDG_CURRENT_DESKTOP")
                .unwrap_or_default()
                .to_ascii_lowercase();
            if desktop.contains("gnome") {
                (Backend::Gnome, gnome)
            } else {
                (Backend::Kde, kde)
            }
        }
    };
    Some((backend, path))
}

/// Last time the record changed, considering SQLite side files which update
/// continuously while the database is in use.
fn freshness(path: &Path) -> Option<SystemTime> {
    let mut latest = fs::metadata(path).and_then(|m| m.modified()).ok();
    if path.file_name()?.to_str()? == "database" {
        for suffix in ["-wal", "-shm"] {
            let mut name = path.file_name()?.to_os_string();
            name.push(suffix);
            let sibling = fs::metadata(path.with_file_name(name))
                .and_then(|m| m.modified())
                .ok();
            latest = latest.max(sibling);
        }
    }
    latest
}

/// Desktop usage for this session's environment, keyed by desktop file ID.
pub fn harvest() -> BTreeMap<String, Usage> {
    match backend() {
        Some((Backend::Kde, _)) => kde_usage(),
        Some((Backend::Gnome, _)) => gnome_usage(),
        None => BTreeMap::new(),
    }
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

// Read-only usage tallies from the desktop environment; see `Backend`.
/// Desktop usage tallies recorded by KDE's activity manager, keyed by
/// desktop file ID. Read through the sqlite3 command-line tool; an absent
/// tool or database yields no entries.
pub fn kde_usage() -> BTreeMap<String, Usage> {
    let Some(path) = kde_state_path() else {
        return BTreeMap::new();
    };
    if !path.exists() {
        return BTreeMap::new();
    }
    let Ok(output) = std::process::Command::new("sqlite3")
        .arg("-readonly")
        .arg(&path)
        .arg(
            "SELECT initiatingAgent, SUM(cachedScore), MAX(lastUpdate) \
             FROM ResourceScoreCache GROUP BY initiatingAgent",
        )
        .output()
    else {
        return BTreeMap::new();
    };
    if !output.status.success() {
        return BTreeMap::new();
    }
    parse_kde(&String::from_utf8_lossy(&output.stdout))
}

/// Whether the `sqlite3` CLI that KDE usage import needs is installed.
pub fn sqlite3_available() -> bool {
    std::process::Command::new("sqlite3")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Map a recorded agent name to a launcher desktop file ID. Agents may name
/// the application without a `.desktop` suffix; anything else (portal paths,
/// URI-like sources) is not an application.
fn application_id(agent: &str) -> Option<String> {
    let id = agent.trim();
    if id.is_empty()
        || id.len() > 256
        || id.contains(':')
        || id.contains('/')
        || id.chars().any(char::is_whitespace)
    {
        return None;
    }
    if id.ends_with(".desktop") {
        Some(id.to_string())
    } else {
        Some(format!("{id}.desktop"))
    }
}

fn parse_kde(rows: &str) -> BTreeMap<String, Usage> {
    let mut usage = BTreeMap::new();
    for line in rows.lines() {
        let mut fields = line.split('|');
        let (agent, score, last) = (fields.next(), fields.next(), fields.next());
        if fields.next().is_some() {
            continue;
        }
        let Some(id) = agent.and_then(application_id) else {
            continue;
        };
        let Some(count) = score.and_then(tally) else {
            continue;
        };
        let Some(last) = last.and_then(tally) else {
            continue;
        };
        usage.insert(id, Usage { count, last });
    }
    usage
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
    fn application_ids_accept_missing_desktop_suffix_and_reject_resources() {
        assert_eq!(
            application_id("org.kde.dolphin"),
            Some("org.kde.dolphin.desktop".to_string())
        );
        assert_eq!(
            application_id("com.brave.Browser.nightly"),
            Some("com.brave.Browser.nightly.desktop".to_string())
        );
        assert_eq!(
            application_id(" firefox-esr.desktop "),
            Some("firefox-esr.desktop".to_string())
        );
        for rejected in [
            "/usr/lib/x86_64-linux-gnu/libexec/xdg-desktop-portal-kde",
            "kcm:kcm_kscreen",
            "preferred://browser",
            "lib with spaces",
            "",
        ] {
            assert_eq!(application_id(rejected), None, "{rejected}");
        }
    }

    #[test]
    fn parses_sqlite_rows_from_kde_statistics() {
        let usage = parse_kde(
            "org.kde.dolphin|994.042029237053484|1793795235\n\
             com.brave.Browser.nightly|33|1793704291\n\
             /usr/lib/libexec/xdg-desktop-portal-kde|255|1793769696\n\
             org.kde.gwenview|8.5|not-a-number\n\
             |12|1793700000\nextra|1|2|3\n",
        );
        assert_eq!(
            usage.get("org.kde.dolphin.desktop"),
            Some(&Usage {
                count: 994,
                last: 1793795235
            })
        );
        assert_eq!(
            usage.get("com.brave.Browser.nightly.desktop"),
            Some(&Usage {
                count: 33,
                last: 1793704291
            })
        );
        assert_eq!(usage.len(), 2);
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
