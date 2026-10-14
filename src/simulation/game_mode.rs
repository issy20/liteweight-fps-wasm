//! Gameplay rules that vary by training mode.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionMode {
    FreeTraining,
    TrackingAim,
}

impl SessionMode {
    pub fn recoil_enabled(self) -> bool {
        self != Self::TrackingAim
    }

    pub fn hip_fire_spread_enabled(self) -> bool {
        self != Self::TrackingAim
    }

    /// Infinite ammo is the default in every mode, but the setting can be
    /// turned off to play with finite magazines.
    pub fn infinite_ammo_enabled(self, setting_enabled: bool) -> bool {
        setting_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::SessionMode;

    #[test]
    fn infinite_ammo_follows_the_setting_in_every_mode() {
        assert!(SessionMode::FreeTraining.infinite_ammo_enabled(true));
        assert!(!SessionMode::FreeTraining.infinite_ammo_enabled(false));
        assert!(SessionMode::TrackingAim.infinite_ammo_enabled(true));
        assert!(!SessionMode::TrackingAim.infinite_ammo_enabled(false));
    }
}
