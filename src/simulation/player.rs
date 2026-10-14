use crate::physics::Aabb;
use glam::{Mat4, Vec3};

/// 視点位置を基準とする一人称プレイヤー。衝突はカプセル相当をAABBで近似する。
pub const EYE_HEIGHT: f32 = 1.6;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_RADIUS: f32 = 0.3;
pub const WALK_SPEED: f32 = 4.0;
pub const DASH_SPEED: f32 = 6.5;
pub const DASH_TRANSITION_SECONDS: f32 = 0.18;
pub const DASH_WEAPON_DROP: f32 = 0.055;
pub const JUMP_VELOCITY: f32 = 4.2;
pub const GRAVITY: f32 = 9.8;
pub const WALK_FOOTSTEP_INTERVAL: f32 = 0.45;
pub const DASH_FOOTSTEP_INTERVAL: f32 = 0.30;
/// 衝突判定の丸め誤差許容。接触距離の拡大ではなく、f32精度の境界での誤接触を防ぐ。
const COLLISION_EPSILON: f32 = 1e-4;
const DEFAULT_STEP_HEIGHT: f32 = 0.22;
const DEFAULT_GROUND_SNAP_DISTANCE: f32 = 0.22;

/// Developer-tunable movement values. Defaults preserve the current game feel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementTuning {
    pub walk_speed: f32,
    pub dash_speed: f32,
    pub jump_velocity: f32,
    pub gravity: f32,
    /// Multiplier applied to movement input while airborne; 0 disables mid-air steering.
    pub air_control: f32,
    /// Maximum vertical rise that grounded movement can step onto.
    pub step_height: f32,
    /// Maximum drop followed while staying grounded on stairs and ramps.
    pub ground_snap_distance: f32,
    /// Horizontal player collision radius, excluding the additional contact margin.
    pub collision_radius: f32,
    /// Extra horizontal clearance from walls and other collision surfaces.
    pub contact_margin: f32,
}

impl Default for MovementTuning {
    fn default() -> Self {
        Self {
            walk_speed: WALK_SPEED,
            dash_speed: DASH_SPEED,
            jump_velocity: JUMP_VELOCITY,
            gravity: GRAVITY,
            air_control: 1.0,
            step_height: DEFAULT_STEP_HEIGHT,
            ground_snap_distance: DEFAULT_GROUND_SNAP_DISTANCE,
            collision_radius: PLAYER_RADIUS,
            contact_margin: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MovementEvents {
    pub landed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct FootstepClock {
    remaining: f32,
}

impl Default for FootstepClock {
    fn default() -> Self {
        Self {
            remaining: WALK_FOOTSTEP_INTERVAL,
        }
    }
}

impl FootstepClock {
    pub fn reset(&mut self) {
        self.remaining = WALK_FOOTSTEP_INTERVAL;
    }

    /// Returns true when a step is due. Motion is supplied from actual horizontal displacement.
    pub fn advance(&mut self, dt: f32, moving: bool, grounded: bool, dashing: bool) -> bool {
        let interval = if dashing {
            DASH_FOOTSTEP_INTERVAL
        } else {
            WALK_FOOTSTEP_INTERVAL
        };
        if !dt.is_finite() || dt <= 0.0 || !moving || !grounded {
            self.remaining = interval;
            return false;
        }
        self.remaining = self.remaining.min(interval) - dt;
        if self.remaining <= 0.0 {
            self.remaining = interval;
            true
        } else {
            false
        }
    }
}

/// 1フレームの入力状態。
#[derive(Debug, Default, Clone, Copy)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub back: bool,
    pub dash: bool,
    pub jump: bool,
}
impl Input {
    /// Movement presses belong to active play, while releases must always clear stale state.
    pub fn should_update_movement_key(pressed: bool, gameplay_active: bool) -> bool {
        !pressed || gameplay_active
    }
    /// Game simulation requires an active, focused window, but not a captured cursor.
    pub fn should_simulate_gameplay(gameplay_active: bool, window_focused: bool) -> bool {
        gameplay_active && window_focused
    }
}

pub struct Player {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub dash_progress: f32,
    pub movement_tuning: MovementTuning,
}
impl Default for Player {
    fn default() -> Self {
        Self {
            position: Vec3::new(0., EYE_HEIGHT, 0.),
            yaw: 0.,
            pitch: 0.,
            velocity: Vec3::ZERO,
            on_ground: true,
            dash_progress: 0.0,
            movement_tuning: MovementTuning::default(),
        }
    }
}
impl Player {
    pub fn new() -> Self {
        Self::default()
    }
    /// 足元の高さ。接地時は0。視点位置と視点高から算出する。
    pub fn foot_height(&self) -> f32 {
        self.position.y - EYE_HEIGHT
    }
    /// 衝突判定で使う身体中心。足元から身体高さの半分の位置。
    pub fn body_center(&self) -> Vec3 {
        Vec3::new(
            self.position.x,
            self.foot_height() + PLAYER_HEIGHT * 0.5,
            self.position.z,
        )
    }
    fn effective_collision_radius(&self) -> f32 {
        self.movement_tuning.collision_radius.max(0.0)
            + self.movement_tuning.contact_margin.max(0.0)
    }
    /// 相対角度を視点へ反映する。感度倍率は呼び出し側で掛ける。
    pub fn look(&mut self, dx: f32, dy: f32) {
        self.yaw += dx;
        self.pitch = (self.pitch - dy).clamp(-1.55, 1.55);
    }
    /// Applies the accepted shot's upward pitch kick directly to the controllable view.
    pub fn apply_recoil(&mut self, pitch_kick: f32) {
        self.pitch = (self.pitch + pitch_kick).clamp(-1.55, 1.55);
    }
    pub fn dash_progress(&self) -> f32 {
        self.dash_progress
    }
    pub fn dash_weapon_offset(&self) -> f32 {
        let progress = self.dash_progress.clamp(0.0, 1.0);
        let eased = progress * progress * (3.0 - 2.0 * progress);
        -eased * DASH_WEAPON_DROP
    }
    /// 視点位置とyaw/pitchから投影×ビュー行列を生成する。
    pub fn view_matrix(&self, aspect: f32) -> Mat4 {
        self.view_matrix_with(aspect, 70.)
    }
    pub fn aim_direction(&self) -> Vec3 {
        let pitch = self.pitch;
        Vec3::new(
            self.yaw.sin() * pitch.cos(),
            pitch.sin(),
            -self.yaw.cos() * pitch.cos(),
        )
    }
    pub fn view_matrix_with(&self, aspect: f32, fov: f32) -> Mat4 {
        let eye = self.position;
        let target = eye + self.aim_direction();
        let projection = Mat4::perspective_rh(fov.to_radians(), aspect, 0.1, 100.);
        projection * Mat4::look_at_rh(eye, target, Vec3::Y)
    }
    /// 入力から水平速度を決めて位置を進める。前方はyawの向き、斜め移動は正規化する。
    pub fn move_with_input(&mut self, input: Input, dt: f32) {
        self.update_horizontal(input, dt);
        if input.jump && self.on_ground {
            self.velocity.y = self.movement_tuning.jump_velocity;
            self.on_ground = false;
        }
        if !self.on_ground {
            self.velocity.y -= self.movement_tuning.gravity * dt;
        }
        self.position += self.velocity * dt;
        if self.position.y <= EYE_HEIGHT {
            self.position.y = EYE_HEIGHT;
            self.velocity.y = 0.;
            self.on_ground = true;
        }
    }
    fn update_horizontal(&mut self, input: Input, dt: f32) -> Vec3 {
        let dash_step = (dt.max(0.0) / DASH_TRANSITION_SECONDS).min(1.0);
        self.dash_progress = if input.dash {
            (self.dash_progress + dash_step).min(1.0)
        } else {
            (self.dash_progress - dash_step).max(0.0)
        };
        let speed = if input.dash {
            self.movement_tuning.dash_speed
        } else {
            self.movement_tuning.walk_speed
        };
        let speed = if self.on_ground {
            speed
        } else {
            speed * self.movement_tuning.air_control.max(0.0)
        };
        let mut input_dir = Vec3::ZERO;
        if input.forward {
            input_dir.z -= 1.;
        }
        if input.back {
            input_dir.z += 1.;
        }
        if input.right && !input.left {
            input_dir.x += 1.;
        }
        if input.left && !input.right {
            input_dir.x -= 1.;
        }
        let input_dir = if input_dir.length_squared() > 0. {
            input_dir.normalize()
        } else {
            input_dir
        };
        let forward = Vec3::new(self.yaw.sin(), 0., -self.yaw.cos());
        let right_vec = Vec3::new(self.yaw.cos(), 0., self.yaw.sin());
        let move_dir = forward * -input_dir.z + right_vec * input_dir.x;
        let target = Vec3::new(move_dir.x * speed, 0., move_dir.z * speed);
        self.velocity.x = target.x;
        self.velocity.z = target.z;
        target
    }
    /// 入力とAABB衝突を考慮して移動する。水平方向は軸ごとにスライディングする。
    pub fn move_with_collision(
        &mut self,
        colliders: &[Aabb],
        input: Input,
        dt: f32,
    ) -> MovementEvents {
        let was_grounded = self.on_ground;
        let horizontal = self.update_horizontal(input, dt);
        if input.jump && self.on_ground {
            self.velocity.y = self.movement_tuning.jump_velocity;
            self.on_ground = false;
        }
        // 水平は軸ごとに移動して衝突を解決し、残った方向へ滑る。
        for axis in [0, 2] {
            if horizontal[axis].abs() <= COLLISION_EPSILON {
                continue;
            }
            let old_position = self.position;
            self.position[axis] += horizontal[axis] * dt;
            if self.collides(colliders, self.body_center()) {
                let current_foot = self.foot_height();
                let step_top = colliders
                    .iter()
                    .filter(|collider| {
                        collider.min.y <= current_foot + COLLISION_EPSILON
                            && collider.max.y > current_foot + COLLISION_EPSILON
                            && collider.max.y <= current_foot + self.movement_tuning.step_height
                            && self.overlaps_player_footprint(collider, self.position)
                    })
                    .map(|collider| collider.max.y)
                    .max_by(f32::total_cmp);
                if self.on_ground && !input.jump {
                    if let Some(step_top) = step_top {
                        self.position.y += step_top - current_foot;
                        if !self.collides(colliders, self.body_center()) {
                            self.velocity.y = 0.0;
                            continue;
                        }
                    }
                }
                self.position = old_position;
                self.velocity[axis] = 0.0;
            }
        }
        if self.on_ground && !input.jump {
            let current_foot = self.foot_height();
            let support = colliders
                .iter()
                .filter(|collider| {
                    collider.max.y <= current_foot + COLLISION_EPSILON
                        && collider.max.y
                            >= current_foot - self.movement_tuning.ground_snap_distance
                        && self.overlaps_player_footprint(collider, self.position)
                })
                .map(|collider| collider.max.y)
                .max_by(f32::total_cmp);
            if let Some(support) = support {
                self.position.y = EYE_HEIGHT + support;
            } else {
                self.on_ground = false;
            }
        }
        if !self.on_ground {
            self.velocity.y -= self.movement_tuning.gravity * dt;
        }
        self.position.y += self.velocity.y * dt;
        if self.collides(colliders, self.body_center()) {
            self.position.y -= self.velocity.y * dt;
            // 落下中の衝突は着地。上昇中の衝突は天井に当たった場合。
            if self.velocity.y < 0. {
                self.on_ground = true;
            }
            self.velocity.y = 0.;
        }
        if self.position.y <= EYE_HEIGHT {
            self.position.y = EYE_HEIGHT;
            self.velocity.y = 0.;
            self.on_ground = true;
        }
        MovementEvents {
            landed: !was_grounded && self.on_ground,
        }
    }
    fn collides(&self, colliders: &[Aabb], center: Vec3) -> bool {
        let half = PLAYER_HEIGHT * 0.5;
        colliders.iter().any(|c| {
            let cy_min = center.y - half;
            let cy_max = center.y + half;
            if cy_max <= c.min.y + COLLISION_EPSILON || cy_min >= c.max.y - COLLISION_EPSILON {
                return false;
            }
            let cx = center.x.clamp(c.min.x, c.max.x);
            let cz = center.z.clamp(c.min.z, c.max.z);
            let dx = center.x - cx;
            let dz = center.z - cz;
            // f32の丸め誤差（壁面からちょうど半径の距離）で接触扱いになるのを防ぐ。
            let r = (self.effective_collision_radius() - COLLISION_EPSILON).max(0.0);
            dx * dx + dz * dz < r * r
        })
    }

    fn overlaps_player_footprint(&self, collider: &Aabb, position: Vec3) -> bool {
        let closest_x = position.x.clamp(collider.min.x, collider.max.x);
        let closest_z = position.z.clamp(collider.min.z, collider.max.z);
        let dx = position.x - closest_x;
        let dz = position.z - closest_z;
        let radius = self.effective_collision_radius();
        dx * dx + dz * dz < radius * radius
    }
}
