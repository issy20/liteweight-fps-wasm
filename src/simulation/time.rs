/// 固定タイムステップの蓄積と更新回数の上限を管理する（仕様§10.3）。
pub struct FixedStep {
    step_seconds: f64,
    accumulated: f64,
    max_updates: u32,
}
impl FixedStep {
    pub fn new(hz: u32, max_updates: u32) -> Self {
        Self {
            step_seconds: 1.0 / hz as f64,
            accumulated: 0.,
            max_updates,
        }
    }
    /// 実時間の経過を蓄積し、固定ステップで更新すべき回数を返す。
    pub fn updates(&mut self, dt: f64) -> u32 {
        self.accumulated += dt;
        let raw = (self.accumulated / self.step_seconds) as u32;
        if raw > self.max_updates {
            // 過負荷時は無限追いつきを防ぐため、残りを捨てて上限まで更新する。
            self.accumulated = 0.;
            self.max_updates
        } else {
            let n = raw;
            if n > 0 {
                self.accumulated -= n as f64 * self.step_seconds;
            }
            n
        }
    }
}
