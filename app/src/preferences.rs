use std::{fs, path::PathBuf};

const GROUP: &str = "lifecycle";
const PAUSE_ON_BATTERY: &str = "pause-on-battery";
const PAUSE_ON_LOW_BATTERY_ONLY: &str = "pause-on-low-battery-only";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatteryPolicyPreferences {
    pub pause_on_battery: bool,
    pub pause_on_low_battery_only: bool,
}

impl Default for BatteryPolicyPreferences {
    fn default() -> Self {
        Self {
            pause_on_battery: true,
            pause_on_low_battery_only: false,
        }
    }
}

pub fn load_battery_policy() -> BatteryPolicyPreferences {
    load_from(path())
}

pub fn save_pause_on_battery(enabled: bool) -> Result<(), String> {
    save_boolean(path(), PAUSE_ON_BATTERY, enabled)
}

pub fn save_pause_on_low_battery_only(enabled: bool) -> Result<(), String> {
    save_boolean(path(), PAUSE_ON_LOW_BATTERY_ONLY, enabled)
}

fn load_from(path: PathBuf) -> BatteryPolicyPreferences {
    let key_file = glib::KeyFile::new();
    if let Err(error) = key_file.load_from_file(&path, glib::KeyFileFlags::NONE) {
        if !error.matches(glib::FileError::Noent) {
            eprintln!("gnomeengine: could not load battery preferences: {error}");
        }
        return BatteryPolicyPreferences::default();
    }

    BatteryPolicyPreferences {
        pause_on_battery: key_file.boolean(GROUP, PAUSE_ON_BATTERY).unwrap_or(true),
        pause_on_low_battery_only: key_file
            .boolean(GROUP, PAUSE_ON_LOW_BATTERY_ONLY)
            .unwrap_or(false),
    }
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

    let temporary = parent.join(format!(".lifecycle-app-{}.tmp", std::process::id()));
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
    use super::{
        load_from, save_boolean, BatteryPolicyPreferences, PAUSE_ON_BATTERY,
        PAUSE_ON_LOW_BATTERY_ONLY,
    };

    #[test]
    fn offline_settings_preserve_both_battery_options() {
        let path = std::env::temp_dir().join(format!(
            "gnomeengine-app-preferences-{}.ini",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        save_boolean(path.clone(), PAUSE_ON_BATTERY, false).unwrap();
        save_boolean(path.clone(), PAUSE_ON_LOW_BATTERY_ONLY, true).unwrap();

        assert_eq!(
            load_from(path.clone()),
            BatteryPolicyPreferences {
                pause_on_battery: false,
                pause_on_low_battery_only: true,
            }
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_preferences_keep_the_current_default() {
        let path = std::env::temp_dir().join(format!(
            "gnomeengine-app-missing-preferences-{}.ini",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        assert_eq!(load_from(path), BatteryPolicyPreferences::default());
    }
}
