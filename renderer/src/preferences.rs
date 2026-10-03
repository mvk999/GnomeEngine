use std::{fs, path::PathBuf};

use crate::lifecycle::LifecyclePolicyConfig;

const GROUP: &str = "lifecycle";
const PAUSE_ON_BATTERY: &str = "pause-on-battery";
const PAUSE_ON_LOW_BATTERY_ONLY: &str = "pause-on-low-battery-only";

pub fn load() -> LifecyclePolicyConfig {
    load_from(path())
}

fn load_from(path: PathBuf) -> LifecyclePolicyConfig {
    let key_file = glib::KeyFile::new();
    match key_file.load_from_file(&path, glib::KeyFileFlags::NONE) {
        Ok(()) => LifecyclePolicyConfig {
            pause_on_battery: key_file.boolean(GROUP, PAUSE_ON_BATTERY).unwrap_or(true),
            pause_on_low_battery_only: key_file
                .boolean(GROUP, PAUSE_ON_LOW_BATTERY_ONLY)
                .unwrap_or(false),
        },
        Err(error) if error.matches(glib::FileError::Noent) => LifecyclePolicyConfig::default(),
        Err(error) => {
            eprintln!("gnomeengine-renderer: could not load preferences: {error}");
            LifecyclePolicyConfig::default()
        }
    }
}

pub fn save_pause_on_battery(enabled: bool) -> Result<(), String> {
    save_boolean(path(), PAUSE_ON_BATTERY, enabled)
}

pub fn save_pause_on_low_battery_only(enabled: bool) -> Result<(), String> {
    save_boolean(path(), PAUSE_ON_LOW_BATTERY_ONLY, enabled)
}

fn save_boolean(path: PathBuf, key: &str, enabled: bool) -> Result<(), String> {
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
    key_file.set_boolean(GROUP, key, enabled);

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
    use super::{load_from, save_boolean, PAUSE_ON_BATTERY, PAUSE_ON_LOW_BATTERY_ONLY};

    #[test]
    fn stored_battery_policy_round_trips() {
        let temp = std::env::temp_dir().join(format!(
            "gnomeengine-preferences-{}.ini",
            std::process::id()
        ));
        save_boolean(temp.clone(), PAUSE_ON_BATTERY, false).unwrap();
        save_boolean(temp.clone(), PAUSE_ON_LOW_BATTERY_ONLY, true).unwrap();
        let config = load_from(temp.clone());
        assert!(!config.pause_on_battery);
        assert!(config.pause_on_low_battery_only);
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
