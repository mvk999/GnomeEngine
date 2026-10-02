use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PauseReason {
    Manual,
    Fullscreen,
    ScreenLocked,
    DisplayOff,
    OnBattery,
    SystemSleep,
}

impl PauseReason {
    pub const ALL: [Self; 6] = [
        Self::Manual,
        Self::Fullscreen,
        Self::ScreenLocked,
        Self::DisplayOff,
        Self::OnBattery,
        Self::SystemSleep,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Fullscreen => "fullscreen",
            Self::ScreenLocked => "screen-locked",
            Self::DisplayOff => "display-off",
            Self::OnBattery => "on-battery",
            Self::SystemSleep => "system-sleep",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|reason| reason.as_str() == value)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LifecyclePolicy {
    pause_reasons: BTreeSet<PauseReason>,
    config: LifecyclePolicyConfig,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LifecyclePolicyConfig {
    pub pause_on_battery: bool,
}

impl Default for LifecyclePolicyConfig {
    fn default() -> Self {
        Self {
            pause_on_battery: true,
        }
    }
}

impl LifecyclePolicy {
    pub fn with_config(config: LifecyclePolicyConfig) -> Self {
        Self {
            pause_reasons: BTreeSet::new(),
            config,
        }
    }

    pub const fn pause_on_battery(&self) -> bool {
        self.config.pause_on_battery
    }

    pub fn set_reason(&mut self, reason: PauseReason, active: bool) -> bool {
        if active {
            self.pause_reasons.insert(reason)
        } else {
            self.pause_reasons.remove(&reason)
        }
    }

    pub fn set_reason_by_name(&mut self, name: &str, active: bool) -> Result<bool, String> {
        let reason =
            PauseReason::parse(name).ok_or_else(|| format!("unknown pause reason: {name}"))?;
        Ok(self.set_reason(reason, active))
    }

    pub fn is_paused(&self) -> bool {
        !self.pause_reasons.is_empty()
    }

    pub fn reasons(&self) -> impl Iterator<Item = PauseReason> + '_ {
        self.pause_reasons.iter().copied()
    }

    pub fn reason_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.reasons().map(PauseReason::as_str)
    }
}

pub fn should_pause_for_fullscreen(wallpaper_outputs: &[u32], fullscreen_outputs: &[u32]) -> bool {
    !wallpaper_outputs.is_empty()
        && wallpaper_outputs
            .iter()
            .all(|output| fullscreen_outputs.contains(output))
}

#[cfg(test)]
mod tests {
    use super::{should_pause_for_fullscreen, LifecyclePolicy, LifecyclePolicyConfig, PauseReason};

    #[test]
    fn empty_reason_set_allows_playback() {
        assert!(!LifecyclePolicy::default().is_paused());
    }

    #[test]
    fn adding_and_removing_manual_reason_changes_policy() {
        let mut policy = LifecyclePolicy::default();

        assert!(policy.set_reason(PauseReason::Manual, true));
        assert!(policy.is_paused());
        assert!(policy.set_reason(PauseReason::Manual, false));
        assert!(!policy.is_paused());
    }

    #[test]
    fn manual_resume_does_not_remove_fullscreen_reason() {
        let mut policy = LifecyclePolicy::default();
        policy.set_reason(PauseReason::Manual, true);
        policy.set_reason(PauseReason::Fullscreen, true);

        assert!(policy.set_reason(PauseReason::Manual, false));
        assert_eq!(policy.reason_names().collect::<Vec<_>>(), ["fullscreen"]);
        assert!(policy.is_paused());
    }

    #[test]
    fn duplicate_add_and_absent_remove_are_no_ops() {
        let mut policy = LifecyclePolicy::default();

        assert!(policy.set_reason(PauseReason::OnBattery, true));
        assert!(!policy.set_reason(PauseReason::OnBattery, true));
        assert!(policy.set_reason(PauseReason::OnBattery, false));
        assert!(!policy.set_reason(PauseReason::OnBattery, false));
    }

    #[test]
    fn unknown_reason_names_are_rejected() {
        assert!(LifecyclePolicy::default()
            .set_reason_by_name("banana", true)
            .is_err());
    }

    #[test]
    fn battery_pause_policy_is_enabled_by_default_and_configurable_internally() {
        assert!(LifecyclePolicy::default().pause_on_battery());
        assert!(!LifecyclePolicy::with_config(LifecyclePolicyConfig {
            pause_on_battery: false,
        })
        .pause_on_battery());
    }

    #[test]
    fn reasons_are_reported_in_stable_order() {
        let mut policy = LifecyclePolicy::default();
        policy.set_reason(PauseReason::OnBattery, true);
        policy.set_reason(PauseReason::Fullscreen, true);

        assert_eq!(
            policy.reason_names().collect::<Vec<_>>(),
            ["fullscreen", "on-battery"]
        );
    }

    #[test]
    fn fullscreen_policy_uses_only_rendered_outputs() {
        assert!(!should_pause_for_fullscreen(&[0], &[]));
        assert!(should_pause_for_fullscreen(&[0], &[0]));
        assert!(!should_pause_for_fullscreen(&[0, 1], &[0]));
        assert!(should_pause_for_fullscreen(&[0, 1], &[0, 1]));
        assert!(!should_pause_for_fullscreen(&[], &[0, 1]));
    }
}
