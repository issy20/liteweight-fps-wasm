/// 内部描画と表示先を分離する。表示先の0サイズは描画休止を表す。
#[derive(Debug, Clone, Copy)]
pub struct FrameLayout {
    internal: (u32, u32),
    resolution_scale: f32,
    display: (u32, u32),
    fill_display: bool,
}
impl FrameLayout {
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        Self::validate(width, height)?;
        Ok(Self {
            internal: (width, height),
            resolution_scale: 1.0,
            display: (width, height),
            fill_display: false,
        })
    }
    fn validate(width: u32, height: u32) -> Result<(), &'static str> {
        if !(1..=4096).contains(&width) || !(1..=4096).contains(&height) {
            return Err("内部解像度は各辺1〜4096pxを指定してください");
        }
        Ok(())
    }
    pub fn set_internal(&mut self, width: u32, height: u32) -> Result<(), &'static str> {
        Self::validate(width, height)?;
        self.internal = (width, height);
        Ok(())
    }
    pub fn internal_size(&self) -> (u32, u32) {
        self.internal
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.display = (width, height);
    }
    /// Match the internal render-target aspect to the display without letterboxing.
    pub fn resize_to_display_aspect(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<(), &'static str> {
        self.resize_to_display_scale(width, height, self.resolution_scale)
    }
    pub fn resize_to_display_scale(
        &mut self,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), &'static str> {
        if !scale.is_finite() || !(0.25..=1.0).contains(&scale) {
            return Err("内部解像度倍率は0.25〜1.0で指定してください");
        }
        self.display = (width, height);
        if width == 0 || height == 0 {
            return Ok(());
        }
        let largest = width.max(height);
        let hardware_scale = (4096.0 / largest as f32).min(1.0);
        let effective_scale = scale.min(hardware_scale);
        let internal_width = (width as f32 * effective_scale).round().max(1.0) as u32;
        let internal_height = (height as f32 * effective_scale).round().max(1.0) as u32;
        Self::validate(internal_width, internal_height)?;
        self.internal = (internal_width, internal_height);
        self.resolution_scale = scale;
        self.fill_display = true;
        Ok(())
    }
    pub fn viewport(&self) -> Option<(u32, u32, u32, u32)> {
        let (dw, dh) = self.display;
        if dw == 0 || dh == 0 {
            return None;
        }
        if self.fill_display {
            return Some((0, 0, dw, dh));
        }
        let (iw, ih) = self.internal;
        let (w, h) = if u64::from(dw) * u64::from(ih) <= u64::from(dh) * u64::from(iw) {
            (dw, (u64::from(dw) * u64::from(ih) / u64::from(iw)) as u32)
        } else {
            ((u64::from(dh) * u64::from(iw) / u64::from(ih)) as u32, dh)
        };
        let (w, h) = (w.max(1), h.max(1));
        Some(((dw - w) / 2, (dh - h) / 2, w, h))
    }
}
