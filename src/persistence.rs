use crate::simulation::game_mode::SessionMode;
use serde::{Deserialize, Serialize};

const RENDER_SCALES: [f32; 4] = [0.5, 0.67, 0.75, 1.0];
const FPS_LIMITS: [u16; 7] = [30, 60, 90, 120, 144, 240, 0];
const MOUSE_DPI_MIN: u32 = 100;
const MOUSE_DPI_MAX: u32 = 6400;
const MOUSE_DPI_STEP: u32 = 100;
/// Radians of view rotation per unit of raw mouse movement, before sensitivity.
const RADIANS_PER_MOUSE_UNIT: f32 = 0.002;

/// Crosshair palette selectable in the settings menu.
pub const CROSSHAIR_COLORS: [&str; 6] = ["WHITE", "GREEN", "CYAN", "YELLOW", "ORANGE", "RED"];
const CROSSHAIR_RGB: [[f32; 3]; 6] = [
    [1.0, 1.0, 1.0],
    [0.35, 1.0, 0.45],
    [0.35, 0.95, 1.0],
    [1.0, 0.92, 0.35],
    [1.0, 0.66, 0.24],
    [1.0, 0.35, 0.32],
];

/// Resolved crosshair geometry and color used by the HUD renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrosshairStyle {
    pub color: [f32; 3],
    pub gap: f32,
    pub length: f32,
    pub thickness: f32,
    pub dot: bool,
}

impl Default for CrosshairStyle {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0],
            gap: 0.008,
            length: 0.012,
            thickness: 0.003,
            dot: true,
        }
    }
}

/// Crosshair color for a palette index, clamped to the valid range.
pub fn crosshair_color_rgb(index: u8) -> [f32; 3] {
    CROSSHAIR_RGB[usize::from(index.min(CROSSHAIR_COLORS.len() as u8 - 1))]
}

/// `localStorage` key used by the web build for settings.
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "liteweight-fps/settings.json";
/// `localStorage` key used by the web build for best records.
#[cfg(target_arch = "wasm32")]
const RECORDS_KEY: &str = "liteweight-fps/records.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Space,
    LeftShift,
    RightShift,
    Escape,
    Digit1,
    Digit2,
    Digit3,
}

impl BindingKey {
    pub fn label(self) -> &'static str {
        match self {
            Self::Space => "SPACE",
            Self::LeftShift => "L-SHIFT",
            Self::RightShift => "R-SHIFT",
            Self::Escape => "ESC",
            Self::Digit1 => "1",
            Self::Digit2 => "2",
            Self::Digit3 => "3",
            key => match key {
                Self::A => "A",
                Self::B => "B",
                Self::C => "C",
                Self::D => "D",
                Self::E => "E",
                Self::F => "F",
                Self::G => "G",
                Self::H => "H",
                Self::I => "I",
                Self::J => "J",
                Self::K => "K",
                Self::L => "L",
                Self::M => "M",
                Self::N => "N",
                Self::O => "O",
                Self::P => "P",
                Self::Q => "Q",
                Self::R => "R",
                Self::S => "S",
                Self::T => "T",
                Self::U => "U",
                Self::V => "V",
                Self::W => "W",
                Self::X => "X",
                Self::Y => "Y",
                Self::Z => "Z",
                _ => unreachable!(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingAction {
    Forward,
    Backward,
    Left,
    Right,
    Dash,
    Jump,
    Reload,
    Pistol,
    AssaultRifle,
    Sniper,
    Pause,
}

impl BindingAction {
    pub const ALL: [Self; 11] = [
        Self::Forward,
        Self::Backward,
        Self::Left,
        Self::Right,
        Self::Dash,
        Self::Jump,
        Self::Reload,
        Self::Pistol,
        Self::AssaultRifle,
        Self::Sniper,
        Self::Pause,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "MOVE FORWARD",
            Self::Backward => "MOVE BACK",
            Self::Left => "MOVE LEFT",
            Self::Right => "MOVE RIGHT",
            Self::Dash => "DASH",
            Self::Jump => "JUMP",
            Self::Reload => "RELOAD",
            Self::Pistol => "PISTOL",
            Self::AssaultRifle => "ASSAULT RIFLE",
            Self::Sniper => "SNIPER",
            Self::Pause => "PAUSE",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct KeyBindings {
    pub forward: BindingKey,
    pub backward: BindingKey,
    pub left: BindingKey,
    pub right: BindingKey,
    pub dash: BindingKey,
    pub jump: BindingKey,
    pub reload: BindingKey,
    pub pistol: BindingKey,
    pub assault_rifle: BindingKey,
    pub sniper: BindingKey,
    pub pause: BindingKey,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            forward: BindingKey::W,
            backward: BindingKey::S,
            left: BindingKey::A,
            right: BindingKey::D,
            dash: BindingKey::LeftShift,
            jump: BindingKey::Space,
            reload: BindingKey::R,
            pistol: BindingKey::Digit1,
            assault_rifle: BindingKey::Digit2,
            sniper: BindingKey::Digit3,
            pause: BindingKey::Escape,
        }
    }
}

impl KeyBindings {
    pub fn get(self, action: BindingAction) -> BindingKey {
        match action {
            BindingAction::Forward => self.forward,
            BindingAction::Backward => self.backward,
            BindingAction::Left => self.left,
            BindingAction::Right => self.right,
            BindingAction::Dash => self.dash,
            BindingAction::Jump => self.jump,
            BindingAction::Reload => self.reload,
            BindingAction::Pistol => self.pistol,
            BindingAction::AssaultRifle => self.assault_rifle,
            BindingAction::Sniper => self.sniper,
            BindingAction::Pause => self.pause,
        }
    }

    pub fn set(&mut self, action: BindingAction, key: BindingKey) -> bool {
        if BindingAction::ALL
            .iter()
            .any(|other| *other != action && self.get(*other) == key)
        {
            return false;
        }
        match action {
            BindingAction::Forward => self.forward = key,
            BindingAction::Backward => self.backward = key,
            BindingAction::Left => self.left = key,
            BindingAction::Right => self.right = key,
            BindingAction::Dash => self.dash = key,
            BindingAction::Jump => self.jump = key,
            BindingAction::Reload => self.reload = key,
            BindingAction::Pistol => self.pistol = key,
            BindingAction::AssaultRifle => self.assault_rifle = key,
            BindingAction::Sniper => self.sniper = key,
            BindingAction::Pause => self.pause = key,
        }
        true
    }

    fn are_unique(self) -> bool {
        BindingAction::ALL
            .into_iter()
            .enumerate()
            .all(|(index, action)| {
                BindingAction::ALL[index + 1..]
                    .iter()
                    .all(|other| self.get(action) != self.get(*other))
            })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct GameSettings {
    pub mouse_sensitivity: f32,
    pub ads_sensitivity: f32,
    pub invert_y: bool,
    pub fov: f32,
    pub fullscreen: bool,
    pub render_scale_index: u8,
    pub fps_limit_index: u8,
    pub key_bindings: KeyBindings,
    pub infinite_ammo: bool,
    pub volume: f32,
    pub hit_marker_strength: f32,
    pub bot_count: u32,
    /// 0 = Static, 1 = Strafe, 2 = Chase, 3 = Mixed.
    pub bot_behavior: u8,
    pub bot_hp: u32,
    pub bot_speed: f32,
    pub bot_respawn_seconds: f32,
    /// Crosshair palette index (`0..=5`), see [`CROSSHAIR_COLORS`].
    pub crosshair_color: u8,
    /// Crosshair arm length multiplier.
    pub crosshair_size: f32,
    /// Crosshair center gap multiplier.
    pub crosshair_gap: f32,
    /// Crosshair stroke thickness multiplier.
    pub crosshair_thickness: f32,
    /// Whether the center dot is drawn.
    pub crosshair_dot: bool,
    /// Mouse DPI, used only for the cm/360 readout.
    pub mouse_dpi: u32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            ads_sensitivity: 0.65,
            invert_y: false,
            fov: 70.0,
            fullscreen: false,
            render_scale_index: 1,
            fps_limit_index: 6,
            key_bindings: KeyBindings::default(),
            infinite_ammo: true,
            volume: 0.7,
            hit_marker_strength: 1.0,
            bot_count: 5,
            bot_behavior: 3,
            bot_hp: 100,
            bot_speed: 1.0,
            bot_respawn_seconds: 2.0,
            crosshair_color: 0,
            crosshair_size: 1.0,
            crosshair_gap: 1.0,
            crosshair_thickness: 1.0,
            crosshair_dot: true,
            mouse_dpi: 800,
        }
    }
}

impl GameSettings {
    pub fn look_multiplier(&self, ads_progress: f32) -> f32 {
        self.mouse_sensitivity * (1.0 + (self.ads_sensitivity - 1.0) * ads_progress.clamp(0.0, 1.0))
    }

    pub fn look_y_sign(&self) -> f32 {
        if self.invert_y {
            -1.0
        } else {
            1.0
        }
    }

    pub fn render_scale(&self) -> f32 {
        RENDER_SCALES[usize::from(self.render_scale_index.min(3))]
    }

    pub fn fps_limit(&self) -> Option<u16> {
        match FPS_LIMITS[usize::from(self.fps_limit_index.min(6))] {
            0 => None,
            fps => Some(fps),
        }
    }

    /// Resolves the crosshair geometry and color from the current settings.
    pub fn crosshair_style(&self) -> CrosshairStyle {
        CrosshairStyle {
            color: crosshair_color_rgb(self.crosshair_color),
            gap: 0.008 * self.crosshair_gap,
            length: 0.012 * self.crosshair_size,
            thickness: 0.003 * self.crosshair_thickness,
            dot: self.crosshair_dot,
        }
    }

    /// Hip-fire centimeters of mouse travel for a full 360-degree turn.
    ///
    /// `cm = (2π / (radians_per_unit * sensitivity)) / dpi * 2.54`.
    pub fn cm_per_360(&self) -> f32 {
        let sensitivity = self.mouse_sensitivity.max(f32::EPSILON);
        let dpi = self.mouse_dpi.max(1) as f32;
        (std::f32::consts::TAU * 2.54) / (RADIANS_PER_MOUSE_UNIT * sensitivity * dpi)
    }

    pub fn toggle_setting(&mut self, setting: usize) {
        match setting {
            0 => self.infinite_ammo = !self.infinite_ammo,
            1 => self.fullscreen = !self.fullscreen,
            2 => self.invert_y = !self.invert_y,
            3 => self.crosshair_dot = !self.crosshair_dot,
            _ => {}
        }
    }

    pub fn adjust_sensitivity(&mut self, setting: usize, direction: f32) {
        match setting {
            0 => {
                self.mouse_sensitivity = (self.mouse_sensitivity + direction * 0.1).clamp(0.1, 5.0)
            }
            1 => self.ads_sensitivity = (self.ads_sensitivity + direction * 0.05).clamp(0.1, 1.0),
            _ => {}
        }
    }

    pub fn adjust_setting(&mut self, setting: usize, direction: f32) {
        if setting < 2 {
            self.adjust_sensitivity(setting, direction);
        } else if setting == 2 {
            self.fov = (self.fov + direction).clamp(60.0, 120.0);
        } else if setting == 3 {
            self.bot_count = (self.bot_count as f32 + direction.signum()).clamp(1.0, 20.0) as u32;
        } else if setting == 4 && direction != 0.0 {
            self.bot_behavior =
                match (self.bot_behavior as i8 + direction.signum() as i8).rem_euclid(4) {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    _ => 3,
                };
        } else if setting == 5 {
            self.volume = (self.volume + direction * 0.05).clamp(0.0, 1.0);
        } else if setting == 6 {
            self.bot_hp =
                (self.bot_hp as f32 + direction.signum() * 10.0).clamp(50.0, 200.0) as u32;
        } else if setting == 7 {
            self.bot_speed = (self.bot_speed + direction * 0.1).clamp(0.5, 2.0);
        } else if setting == 8 {
            self.bot_respawn_seconds =
                (self.bot_respawn_seconds + direction * 0.25).clamp(0.5, 10.0);
        } else if setting == 9 && direction != 0.0 {
            self.render_scale_index = (i16::from(self.render_scale_index)
                + direction.signum() as i16)
                .rem_euclid(4) as u8;
        } else if setting == 10 && direction != 0.0 {
            self.fps_limit_index =
                (i16::from(self.fps_limit_index) + direction.signum() as i16).rem_euclid(7) as u8;
        } else if setting == 11 {
            self.hit_marker_strength = (self.hit_marker_strength + direction * 0.1).clamp(0.0, 1.0);
        } else if setting == 12 {
            self.crosshair_size = (self.crosshair_size + direction * 0.1).clamp(0.5, 2.0);
        } else if setting == 13 {
            self.crosshair_gap = (self.crosshair_gap + direction * 0.1).clamp(0.0, 2.0);
        } else if setting == 14 {
            self.crosshair_thickness = (self.crosshair_thickness + direction * 0.1).clamp(0.5, 2.0);
        } else if setting == 15 && direction != 0.0 {
            self.crosshair_color = (i16::from(self.crosshair_color) + direction.signum() as i16)
                .rem_euclid(CROSSHAIR_COLORS.len() as i16) as u8;
        } else if setting == 16 && direction != 0.0 {
            let delta = direction.signum() as i32 * MOUSE_DPI_STEP as i32;
            self.mouse_dpi = (self.mouse_dpi as i32 + delta)
                .clamp(MOUSE_DPI_MIN as i32, MOUSE_DPI_MAX as i32)
                as u32;
        }
    }

    pub fn from_json(json: &str) -> Self {
        let Ok(settings) = serde_json::from_str::<Self>(json) else {
            return Self::default();
        };
        if !(0.1..=5.0).contains(&settings.mouse_sensitivity)
            || !(0.1..=1.0).contains(&settings.ads_sensitivity)
            || !(60.0..=120.0).contains(&settings.fov)
            || !(0.0..=1.0).contains(&settings.volume)
            || !(0.0..=1.0).contains(&settings.hit_marker_strength)
            || !(1..=20).contains(&settings.bot_count)
            || settings.bot_behavior > 3
            || !(50..=200).contains(&settings.bot_hp)
            || !(0.5..=2.0).contains(&settings.bot_speed)
            || !(0.5..=10.0).contains(&settings.bot_respawn_seconds)
            || settings.render_scale_index > 3
            || settings.fps_limit_index > 6
            || settings.crosshair_color as usize >= CROSSHAIR_COLORS.len()
            || !(0.5..=2.0).contains(&settings.crosshair_size)
            || !(0.0..=2.0).contains(&settings.crosshair_gap)
            || !(0.5..=2.0).contains(&settings.crosshair_thickness)
            || !(MOUSE_DPI_MIN..=MOUSE_DPI_MAX).contains(&settings.mouse_dpi)
            || !settings.key_bindings.are_unique()
        {
            return Self::default();
        }
        settings
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Self {
        let Some(json) =
            web_storage().and_then(|storage| storage.get_item(STORAGE_KEY).ok().flatten())
        else {
            return Self::default();
        };
        Self::from_json(&json)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save(&self) -> Result<(), String> {
        let storage =
            web_storage().ok_or_else(|| "設定保存先(localStorage)を利用できません".to_owned())?;
        let data = serde_json::to_string(self).map_err(|e| e.to_string())?;
        storage
            .set_item(STORAGE_KEY, &data)
            .map_err(|_| "設定の保存に失敗しました".to_owned())
    }
}

/// Best result for a single training mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ModeRecord {
    pub kills: u32,
    pub accuracy: f32,
    pub headshot_rate: f32,
}

/// Persistent best results per mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Records {
    pub free_training: ModeRecord,
    pub tracking_aim: ModeRecord,
}

impl Records {
    pub fn get(&self, mode: SessionMode) -> ModeRecord {
        match mode {
            SessionMode::FreeTraining => self.free_training,
            SessionMode::TrackingAim => self.tracking_aim,
        }
    }

    fn get_mut(&mut self, mode: SessionMode) -> &mut ModeRecord {
        match mode {
            SessionMode::FreeTraining => &mut self.free_training,
            SessionMode::TrackingAim => &mut self.tracking_aim,
        }
    }

    /// Stores any improved metric and returns whether the record improved.
    pub fn submit(&mut self, mode: SessionMode, result: ModeRecord) -> bool {
        const EPSILON: f32 = 1e-4;
        let record = self.get_mut(mode);
        let improved = result.kills > record.kills
            || result.accuracy > record.accuracy + EPSILON
            || result.headshot_rate > record.headshot_rate + EPSILON;
        record.kills = record.kills.max(result.kills);
        record.accuracy = record.accuracy.max(result.accuracy);
        record.headshot_rate = record.headshot_rate.max(result.headshot_rate);
        improved
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Self {
        let Some(json) =
            web_storage().and_then(|storage| storage.get_item(RECORDS_KEY).ok().flatten())
        else {
            return Self::default();
        };
        serde_json::from_str(&json).unwrap_or_default()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save(&self) -> Result<(), String> {
        let storage =
            web_storage().ok_or_else(|| "記録保存先(localStorage)を利用できません".to_owned())?;
        let data = serde_json::to_string(self).map_err(|e| e.to_string())?;
        storage
            .set_item(RECORDS_KEY, &data)
            .map_err(|_| "記録の保存に失敗しました".to_owned())
    }
}

#[cfg(target_arch = "wasm32")]
fn web_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

// The native build keeps the original atomic file storage so the ported logic
// can be exercised by `cargo test` without a browser.
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::GameSettings;
    use std::{
        fs::{self, OpenOptions},
        io::{self, Write},
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn write_atomically(path: &Path, temp_path: &Path, data: &[u8]) -> io::Result<()> {
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp_path)?;
        let result = (|| {
            temp_file.write_all(data)?;
            temp_file.sync_all()?;
            drop(temp_file);
            replace_file(temp_path, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp_path);
        }
        result
    }

    fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
        fs::rename(source, destination)
    }

    fn write_file_atomically(path: &Path, data: &[u8]) -> io::Result<()> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name")
        })?;
        for _ in 0..16 {
            let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let temp_path = parent.join(format!(
                ".{}.{}.{}.tmp",
                file_name.to_string_lossy(),
                std::process::id(),
                sequence
            ));
            match write_atomically(path, &temp_path, data) {
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                result => return result,
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not reserve a temporary file for atomic write",
        ))
    }

    impl GameSettings {
        pub fn load() -> Self {
            let Some(path) = Self::path() else {
                return Self::default();
            };
            Self::load_from(&path)
        }

        pub fn save(&self) -> Result<(), String> {
            let path = Self::path().ok_or_else(|| "設定保存先を特定できません".to_owned())?;
            Self::save_to(&path, self)
        }

        pub(crate) fn load_from(path: &Path) -> Self {
            fs::read_to_string(path)
                .map(|json| Self::from_json(&json))
                .unwrap_or_default()
        }

        pub(crate) fn save_to(path: &Path, settings: &Self) -> Result<(), String> {
            let parent = path
                .parent()
                .ok_or_else(|| "設定保存先が不正です".to_owned())?;
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let data = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
            write_file_atomically(path, &data).map_err(|e| e.to_string())
        }

        fn path() -> Option<PathBuf> {
            Some(config_root()?.join("liteweight-fps").join("settings.json"))
        }
    }

    impl super::Records {
        pub fn load() -> Self {
            let Some(path) = records_path() else {
                return Self::default();
            };
            fs::read_to_string(path)
                .ok()
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default()
        }

        pub fn save(&self) -> Result<(), String> {
            let path = records_path().ok_or_else(|| "記録保存先を特定できません".to_owned())?;
            let parent = path
                .parent()
                .ok_or_else(|| "記録保存先が不正です".to_owned())?;
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let data = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
            write_file_atomically(&path, &data).map_err(|e| e.to_string())
        }
    }

    fn records_path() -> Option<PathBuf> {
        Some(config_root()?.join("liteweight-fps").join("records.json"))
    }

    fn config_root() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        let root = std::env::var_os("APPDATA").map(PathBuf::from)?;
        #[cfg(target_os = "macos")]
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)?
            .join("Library/Application Support");
        #[cfg(all(unix, not(target_os = "macos")))]
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(root)
    }

    #[cfg(test)]
    mod tests {
        use super::GameSettings;
        use std::fs;

        #[test]
        fn settings_save_and_load_round_trip_from_local_file() {
            let path = std::env::temp_dir().join(format!(
                "liteweight-fps-settings-{}.json",
                std::process::id()
            ));
            let settings = GameSettings {
                mouse_sensitivity: 1.7,
                ads_sensitivity: 0.55,
                fov: 90.0,
                volume: 0.42,
                ..GameSettings::default()
            };
            GameSettings::save_to(&path, &settings).unwrap();
            assert_eq!(GameSettings::load_from(&path), settings);
            fs::remove_file(path).unwrap();
        }

        #[test]
        fn saving_settings_replaces_an_existing_file() {
            let path = std::env::temp_dir().join(format!(
                "liteweight-fps-settings-replace-{}.json",
                std::process::id()
            ));
            let mut settings = GameSettings::default();
            GameSettings::save_to(&path, &settings).unwrap();
            settings.mouse_sensitivity = 2.0;

            GameSettings::save_to(&path, &settings).unwrap();

            assert_eq!(GameSettings::load_from(&path).mouse_sensitivity, 2.0);
            fs::remove_file(path).unwrap();
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{GameSettings, ModeRecord, Records};
    use crate::simulation::game_mode::SessionMode;

    #[test]
    fn crosshair_defaults_use_the_reference_geometry() {
        let settings = GameSettings::default();
        let style = settings.crosshair_style();
        assert_eq!(style.color, [1.0, 1.0, 1.0]);
        assert!((style.gap - 0.008).abs() < 1e-6);
        assert!((style.length - 0.012).abs() < 1e-6);
        assert!((style.thickness - 0.003).abs() < 1e-6);
        assert!(style.dot);
    }

    #[test]
    fn cm_per_360_halves_when_sensitivity_doubles() {
        let base = GameSettings {
            mouse_sensitivity: 1.0,
            mouse_dpi: 800,
            ..GameSettings::default()
        };
        assert!((base.cm_per_360() - 9.974).abs() < 0.05);
        let twice = GameSettings {
            mouse_sensitivity: 2.0,
            ..base
        };
        assert!((twice.cm_per_360() - base.cm_per_360() / 2.0).abs() < 1e-4);
    }

    #[test]
    fn crosshair_and_dpi_adjustments_are_bounded() {
        let mut settings = GameSettings::default();
        settings.adjust_setting(12, 1.0);
        assert!(settings.crosshair_size > 1.0);
        settings.adjust_setting(16, -1.0);
        assert_eq!(settings.mouse_dpi, 700);
        settings.toggle_setting(3);
        assert!(!settings.crosshair_dot);
    }

    #[test]
    fn records_keep_the_best_per_mode_independently() {
        let mut records = Records::default();
        let good = ModeRecord {
            kills: 5,
            accuracy: 0.4,
            headshot_rate: 0.2,
        };
        assert!(records.submit(SessionMode::FreeTraining, good));
        let worse = ModeRecord {
            kills: 3,
            accuracy: 0.3,
            headshot_rate: 0.1,
        };
        assert!(!records.submit(SessionMode::FreeTraining, worse));
        assert_eq!(records.get(SessionMode::FreeTraining).kills, 5);
        assert_eq!(records.get(SessionMode::TrackingAim), ModeRecord::default());
        assert!(records.submit(
            SessionMode::TrackingAim,
            ModeRecord {
                kills: 1,
                accuracy: 0.9,
                headshot_rate: 0.0,
            },
        ));
        assert_eq!(records.get(SessionMode::TrackingAim).accuracy, 0.9);
    }

    #[test]
    fn from_json_falls_back_to_defaults_on_invalid_values() {
        assert_eq!(
            GameSettings::from_json(r#"{"mouse_sensitivity":99.0}"#),
            GameSettings::default()
        );
        assert_eq!(GameSettings::from_json("not json"), GameSettings::default());
    }

    #[test]
    fn from_json_accepts_a_valid_round_trip() {
        let settings = GameSettings {
            mouse_sensitivity: 2.4,
            fov: 110.0,
            bot_count: 12,
            hit_marker_strength: 0.4,
            ..GameSettings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert_eq!(GameSettings::from_json(&json), settings);
    }
}
