use std::{fs, path::PathBuf};

use crate::lifecycle::LifecyclePolicyConfig;

const GROUP: &str = "lifecycle";
const PAUSE_ON_BATTERY: &str = "pause-on-battery";

pub fn load() -> LifecyclePolicyConfig {
    load_from(path())
}

fn load_from(path: PathBuf) -> LifecyclePolicyConfig {
    let key_file = glib::KeyFile::new();
    match key_file.load_from_file(&path, glib::KeyFileFlags::NONE) {
        Ok(()) => LifecyclePolicyConfig {
            pause_on_battery: key_file.boolean(GROUP, PAUSE_ON_BATTERY).unwrap_or(true),
        },
        Err(error) if error.matches(glib::FileError::Noent) => LifecyclePolicyConfig::default(),
        Err(error) => {
            eprintln!("gnomeengine-renderer: could not load preferences: {error}");
            LifecyclePolicyConfig::default()
        }
    }
}

pub fn save_pause_on_battery(enabled: bool) -> Result<(), String> {
    save(path(), enabled)
}

fn save(path: PathBuf, enabled: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "preferences path has no parent directory".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| format!("create preferences directory: {error}"))?;

    let key_file = glib::KeyFile::new();
    if path.exists() {
        key_file
            .load_from_file(&path, glib::KeyFileFlags::NONE)
            .map_err(|error| format!("read preferences: {error}"))?;
    }
    key_file.set_boolean(GROUP, PAUSE_ON_BATTERY, enabled);

    let temporary = parent.join(format!(".lifecycle-{}.tmp", std::process::id()));
    key_file
        .save_to_file(&temporary)
        .map_err(|error| format!("write temporary preferences: {error}"))?;
    fs::rename(&temporary, &path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("commit preferences: {error}")
    })
}

fn path() -> PathBuf {
    glib::user_config_dir()
        .join("gnomeengine")
        .join("lifecycle.ini")
}

#[cfg(test)]
mod tests {
    use super::{load_from, save};

    #[test]
    fn stored_battery_policy_round_trips() {
        let temp = std::env::temp_dir().join(format!(
            "gnomeengine-preferences-{}.ini",
            std::process::id()
        ));
        save(temp.clone(), false).unwrap();
        assert!(!load_from(temp.clone()).pause_on_battery);
        std::fs::remove_file(temp).unwrap();
    }

    #[test]
    fn missing_setting_uses_the_conservative_enabled_default() {
        let temp = std::env::temp_dir().join(format!(
            "gnomeengine-missing-preferences-{}.ini",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&temp);
        assert!(load_from(temp).pause_on_battery);
    }
}
