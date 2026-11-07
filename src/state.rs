use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub portal_shortcut: bool,
    pub shortcut: String,
    pub theme: String,
    pub accent: String,
    pub width: i32,
    pub results: usize,
    pub compact: bool,
    pub show_settings_button: bool,
    pub history: bool,
    pub descriptions: bool,
    pub fuzzy: bool,
    pub clear_on_show: bool,
    pub font_size: i32,
    pub system_usage: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            portal_shortcut: false,
            shortcut: "ALT+space".into(),
            theme: "dark".into(),
            accent: "#8da9ff".into(),
            width: 680,
            results: 8,
            compact: false,
            show_settings_button: false,
            history: true,
            descriptions: true,
            fuzzy: true,
            clear_on_show: true,
            font_size: 19,
            system_usage: true,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !["dark", "light", "system"].contains(&self.theme.as_str())
            || self.shortcut.is_empty()
            || self.shortcut.len() > 128
            || self.shortcut.chars().any(char::is_control)
            || !(360..=1200).contains(&self.width)
            || !(1..=20).contains(&self.results)
            || !(12..=32).contains(&self.font_size)
            || self.accent.len() != 7
            || !self.accent.starts_with('#')
            || !self.accent[1..].bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err("Invalid theme, size, result count, or accent color".into());
        }
        Ok(())
    }
}
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Usage {
    pub count: u64,
    pub last: u64,
}
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub favorites: BTreeSet<String>,
    pub usage: BTreeMap<String, Usage>,
    pub ignored: BTreeSet<String>,
}
pub fn directory(kind: &str) -> PathBuf {
    let (key, fallback) = match kind {
        "state" => ("XDG_STATE_HOME", ".local/state"),
        _ => ("XDG_CONFIG_HOME", ".config"),
    };
    std::env::var_os(key)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback)
        })
        .join("luma-launcher")
}
pub fn load<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T, String> {
    use std::io::Read;
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(T::default()),
        Err(e) => return Err(e.to_string()),
    };
    let mut contents = String::new();
    file.take(4 * 1024 * 1024 + 1)
        .read_to_string(&mut contents)
        .map_err(|e| e.to_string())?;
    if contents.len() > 4 * 1024 * 1024 {
        return Err(format!(
            "{} exceeds the 4 MiB settings limit",
            path.display()
        ));
    }
    toml::from_str(&contents).map_err(|e| format!("{}: {e}", path.display()))
}
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let data = toml::to_string_pretty(value).map_err(|e| e.to_string())?;
    atomic_write(path, data.as_bytes())
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    use std::{
        io::Write,
        os::unix::fs::OpenOptionsExt,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    // A failed exclusive open never grants ownership of the existing file.
    let (temporary, mut file) = loop {
        let temporary = path.with_extension(format!(
            "{}.{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    };
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        fs::File::open(parent)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e: io::Error| e.to_string())
}

pub fn autostart_entry(executable: &Path) -> Result<String, String> {
    let value = executable.to_str().ok_or("Executable path must be UTF-8")?;
    if !executable.is_absolute()
        || value.chars().any(char::is_control)
        || value.contains(['%', '='])
    {
        return Err(
            "Executable path must be absolute and contain no controls, percent, or equals signs"
                .into(),
        );
    }
    // shortcut: reject % in executable paths until GIO supports resolving field codes there.
    let quoted = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
    let file = glib::KeyFile::new();
    file.set_string("Desktop Entry", "Type", "Application");
    file.set_string("Desktop Entry", "Name", "Luma Launcher");
    file.set_string(
        "Desktop Entry",
        "Exec",
        &format!("\"{quoted}\" --background"),
    );
    file.set_boolean("Desktop Entry", "Terminal", false);
    Ok(file.to_data().into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_button_is_opt_in_and_preference_round_trips() {
        assert!(!Config::default().show_settings_button);
        assert!(
            !toml::from_str::<Config>("theme = 'light'")
                .unwrap()
                .show_settings_button
        );
        for show_settings_button in [true, false] {
            let config = Config {
                show_settings_button,
                ..Config::default()
            };
            let saved = toml::to_string(&config).unwrap();
            assert_eq!(
                toml::from_str::<Config>(&saved)
                    .unwrap()
                    .show_settings_button,
                show_settings_button
            );
        }
    }

    #[test]
    fn config_validation() {
        assert!(Config::default().validate().is_ok());
        let c = Config {
            accent: ";bad".into(),
            ..Config::default()
        };
        assert!(c.validate().is_err());
        assert!(toml::from_str::<Config>("invalid=").is_err());
    }
    #[test]
    fn persistence() {
        let dir = std::env::temp_dir().join(format!("luma-test-{}", std::process::id()));
        let path = dir.join("state.toml");
        let mut s = State::default();
        s.favorites.insert("test.desktop".into());
        save(&path, &s).unwrap();
        assert!(load::<State>(&path)
            .unwrap()
            .favorites
            .contains("test.desktop"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stale_temporary_file_is_not_removed_or_blocks_save() {
        let dir = std::env::temp_dir().join(format!("luma-stale-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let stale = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&stale, "unrelated pending write").unwrap();
        let result = save(&path, &Config::default());
        assert_eq!(
            fs::read_to_string(&stale).unwrap(),
            "unrelated pending write"
        );
        assert!(result.is_ok(), "{result:?}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn concurrent_writes_remain_valid_and_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("luma-concurrent-{}", std::process::id()));
        let path = dir.join("config.toml");
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || {
                    save(
                        &path,
                        &Config {
                            width: 600 + i,
                            ..Config::default()
                        },
                    )
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap().unwrap();
        }
        load::<Config>(&path).unwrap().validate().unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn oversized_and_corrupt_settings_are_preserved() {
        let dir = std::env::temp_dir().join(format!("luma-corrupt-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let file = fs::File::create(&path).unwrap();
        file.set_len(4 * 1024 * 1024 + 1).unwrap();
        assert!(load::<Config>(&path).err().unwrap().contains("4 MiB"));
        fs::write(&path, "not valid toml").unwrap();
        assert!(load::<Config>(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "not valid toml");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_shortcuts_are_rejected() {
        for shortcut in ["", "ALT+\nspace", "\0"] {
            assert!(Config {
                shortcut: shortcut.into(),
                ..Config::default()
            }
            .validate()
            .is_err());
        }
    }
}
