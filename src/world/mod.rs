use crate::physics::Aabb;
use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}
pub struct Scene {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}
impl Scene {
    pub fn cube(&mut self, center: Vec3, size: Vec3, color: [f32; 3]) {
        let corners = [
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., 1., -1.],
            [-1., 1., -1.],
            [-1., -1., 1.],
            [1., -1., 1.],
            [1., 1., 1.],
            [-1., 1., 1.],
        ];
        for (face, shade) in [
            ([0, 1, 2, 3], 0.65),
            ([5, 4, 7, 6], 0.85),
            ([4, 0, 3, 7], 0.7),
            ([1, 5, 6, 2], 0.8),
            ([3, 2, 6, 7], 1.0),
            ([4, 5, 1, 0], 0.5),
        ] {
            let base = self.vertices.len() as u16;
            for i in face {
                self.vertices.push(Vertex {
                    position: (center + Vec3::from(corners[i]) * size * 0.5).to_array(),
                    color: [color[0] * shade, color[1] * shade, color[2] * shade, 1.0],
                });
            }
            self.indices
                .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    fn quad(&mut self, points: [Vec3; 4], color: [f32; 3]) {
        let base = self.vertices.len() as u16;
        for point in points {
            self.vertices.push(Vertex {
                position: point.to_array(),
                color: [color[0], color[1], color[2], 1.0],
            });
        }
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn triangle(&mut self, points: [Vec3; 3], color: [f32; 3]) {
        let base = self.vertices.len() as u16;
        for point in points {
            self.vertices.push(Vertex {
                position: point.to_array(),
                color: [color[0], color[1], color[2], 1.0],
            });
        }
        self.indices.extend([base, base + 1, base + 2]);
    }

    fn ramp_z(&mut self, x: f32, low_z: f32, width: f32, length: f32, rise: f32, color: [f32; 3]) {
        let left = x - width * 0.5;
        let right = x + width * 0.5;
        let high_z = low_z - length;
        let low_left = Vec3::new(left, 0.0, low_z);
        let low_right = Vec3::new(right, 0.0, low_z);
        let high_left = Vec3::new(left, rise, high_z);
        let high_right = Vec3::new(right, rise, high_z);
        let back_left = Vec3::new(left, 0.0, high_z);
        let back_right = Vec3::new(right, 0.0, high_z);
        self.quad(
            [low_left, low_right, high_right, high_left],
            [color[0] * 1.15, color[1] * 1.15, color[2] * 1.15],
        );
        self.triangle([low_left, high_left, back_left], color);
        self.triangle([low_right, back_right, high_right], color);
        self.quad(
            [back_left, back_right, high_right, high_left],
            [color[0] * 0.72, color[1] * 0.72, color[2] * 0.72],
        );
        self.quad(
            [low_right, low_left, back_left, back_right],
            [color[0] * 0.58, color[1] * 0.58, color[2] * 0.58],
        );
    }
}

const STAIR_X: f32 = -22.5;
const STAIR_WIDTH: f32 = 3.2;
const STAIR_TREAD: f32 = 0.5;
const STAIR_RISE: f32 = 0.18;
const STAIR_COUNT: usize = 10;
const ROUTE_LOW_Z: f32 = 4.5;
const RAMP_X: f32 = 22.5;
const RAMP_WIDTH: f32 = 3.2;
const RAMP_LENGTH: f32 = 8.0;
const RAMP_RISE: f32 = 1.8;
const RAMP_SEGMENTS: usize = 32;
/// 静的な50m角のアリーナ。移動・衝突・AIには依存しない。
pub fn arena() -> Scene {
    arena_scaled(1.0)
}

const TRACKING_RANGE_WIDTH: f32 = 40.0;
const TRACKING_RANGE_DEPTH: f32 = 24.0;
const TRACKING_RANGE_WALL_HEIGHT: f32 = 8.0;
const TRACKING_RANGE_WALL_THICKNESS: f32 = 0.5;

/// A compact, enclosed, cover-free range for continuous moving-target practice.
pub fn tracking_range() -> Scene {
    let mut scene = Scene {
        vertices: Vec::with_capacity(120),
        indices: Vec::with_capacity(180),
    };
    scene.cube(
        Vec3::new(0.0, -0.25, 0.0),
        Vec3::new(TRACKING_RANGE_WIDTH, 0.5, TRACKING_RANGE_DEPTH),
        [0.35, 0.4, 0.45],
    );
    for sign in [-1.0, 1.0] {
        scene.cube(
            Vec3::new(
                sign * (TRACKING_RANGE_WIDTH - TRACKING_RANGE_WALL_THICKNESS) * 0.5,
                TRACKING_RANGE_WALL_HEIGHT * 0.5,
                0.0,
            ),
            Vec3::new(
                TRACKING_RANGE_WALL_THICKNESS,
                TRACKING_RANGE_WALL_HEIGHT,
                TRACKING_RANGE_DEPTH,
            ),
            [0.5, 0.55, 0.6],
        );
        scene.cube(
            Vec3::new(
                0.0,
                TRACKING_RANGE_WALL_HEIGHT * 0.5,
                sign * (TRACKING_RANGE_DEPTH - TRACKING_RANGE_WALL_THICKNESS) * 0.5,
            ),
            Vec3::new(
                TRACKING_RANGE_WIDTH,
                TRACKING_RANGE_WALL_HEIGHT,
                TRACKING_RANGE_WALL_THICKNESS,
            ),
            [0.5, 0.55, 0.6],
        );
    }
    scene
}

/// Floor and four perimeter walls; no interior cover objects.
pub fn tracking_colliders() -> Vec<Aabb> {
    let half_width = TRACKING_RANGE_WIDTH * 0.5;
    let half_depth = TRACKING_RANGE_DEPTH * 0.5;
    let wall = TRACKING_RANGE_WALL_THICKNESS;
    vec![
        Aabb::new(
            Vec3::new(-half_width, -0.5, -half_depth),
            Vec3::new(half_width, 0.0, half_depth),
        ),
        Aabb::new(
            Vec3::new(-half_width, 0.0, -half_depth),
            Vec3::new(-half_width + wall, TRACKING_RANGE_WALL_HEIGHT, half_depth),
        ),
        Aabb::new(
            Vec3::new(half_width - wall, 0.0, -half_depth),
            Vec3::new(half_width, TRACKING_RANGE_WALL_HEIGHT, half_depth),
        ),
        Aabb::new(
            Vec3::new(-half_width, 0.0, -half_depth),
            Vec3::new(half_width, TRACKING_RANGE_WALL_HEIGHT, -half_depth + wall),
        ),
        Aabb::new(
            Vec3::new(-half_width, 0.0, half_depth - wall),
            Vec3::new(half_width, TRACKING_RANGE_WALL_HEIGHT, half_depth),
        ),
    ]
}

/// BOT数1〜20に合わせて、50m角を34m〜110mへ水平スケールする。
pub fn arena_scale_for_bot_count(count: u32) -> f32 {
    0.68 + (count.clamp(1, 20) - 1) as f32 * 0.08
}

pub fn arena_scaled(scale: f32) -> Scene {
    let scale = if scale.is_finite() {
        scale.clamp(0.68, 2.2)
    } else {
        1.0
    };
    let horizontal = |v: Vec3| Vec3::new(v.x * scale, v.y, v.z * scale);
    let mut s = Scene {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    s.cube(
        horizontal(Vec3::new(0., -0.25, 0.)),
        horizontal(Vec3::new(50., 0.5, 50.)),
        [0.35, 0.4, 0.45],
    );
    for x in [-25., 25.] {
        s.cube(
            horizontal(Vec3::new(x, 6., 0.)),
            horizontal(Vec3::new(0.5, 12., 50.)),
            [0.5, 0.55, 0.6],
        );
        s.cube(
            horizontal(Vec3::new(0., 6., x)),
            horizontal(Vec3::new(50., 12., 0.5)),
            [0.5, 0.55, 0.6],
        );
    }
    for x in [-12., 0., 12.] {
        for z in [-8., 8.] {
            s.cube(
                horizontal(Vec3::new(x, 2.5, z)),
                horizontal(Vec3::new(3., 5., 3.)),
                [0.3, 0.5, 0.55],
            );
        }
    }
    for index in 0..STAIR_COUNT {
        let height = STAIR_RISE * (index + 1) as f32;
        let z = ROUTE_LOW_Z - (index as f32 + 0.5) * STAIR_TREAD;
        s.cube(
            horizontal(Vec3::new(STAIR_X, height * 0.5, z)),
            horizontal(Vec3::new(STAIR_WIDTH, height, STAIR_TREAD)),
            [0.42, 0.56, 0.58],
        );
    }
    let stair_landing_start = ROUTE_LOW_Z - STAIR_COUNT as f32 * STAIR_TREAD;
    s.cube(
        horizontal(Vec3::new(
            STAIR_X,
            STAIR_COUNT as f32 * STAIR_RISE * 0.5,
            stair_landing_start - 1.5,
        )),
        horizontal(Vec3::new(STAIR_WIDTH, STAIR_COUNT as f32 * STAIR_RISE, 3.0)),
        [0.36, 0.49, 0.52],
    );

    let ramp_low_z = ROUTE_LOW_Z;
    let ramp_high_z = ramp_low_z - RAMP_LENGTH;
    s.ramp_z(
        RAMP_X * scale,
        ramp_low_z * scale,
        RAMP_WIDTH * scale,
        RAMP_LENGTH * scale,
        RAMP_RISE,
        [0.48, 0.60, 0.57],
    );
    s.cube(
        horizontal(Vec3::new(RAMP_X, RAMP_RISE * 0.5, ramp_high_z - 1.0)),
        horizontal(Vec3::new(RAMP_WIDTH, RAMP_RISE, 2.0)),
        [0.36, 0.49, 0.52],
    );
    s
}

/// M0で固定した負荷比較用シーン。
pub fn scene(dummies: u32) -> Scene {
    let mut s = arena();
    for i in 0..dummies {
        let x = (i % 5) as f32 * 7. - 14.;
        let z = (i / 5) as f32 * 5. - 10.;
        s.cube(
            Vec3::new(x, 0.8, z),
            Vec3::new(0.7, 1.6, 0.5),
            [0.9, 0.25, 0.12],
        );
        s.cube(Vec3::new(x, 1.85, z), Vec3::splat(0.5), [1., 0.6, 0.2]);
    }
    s
}

/// アリーナの床・壁・遮蔽物をAABBコライダーとして返す。床は範囲外落下防止のための平面。
pub fn arena_colliders() -> Vec<Aabb> {
    arena_colliders_scaled(1.0)
}

pub fn arena_colliders_scaled(scale: f32) -> Vec<Aabb> {
    let scale = if scale.is_finite() {
        scale.clamp(0.68, 2.2)
    } else {
        1.0
    };
    let mut colliders = Vec::new();
    colliders.push(Aabb::new(
        Vec3::new(-25. * scale, -0.5, -25. * scale),
        Vec3::new(25. * scale, 0., 25. * scale),
    ));
    for x in [-25., 25.] {
        colliders.push(Aabb::new(
            Vec3::new(x * scale - 0.25 * scale, 0., -25. * scale),
            Vec3::new(x * scale + 0.25 * scale, 12., 25. * scale),
        ));
        colliders.push(Aabb::new(
            Vec3::new(-25. * scale, 0., x * scale - 0.25 * scale),
            Vec3::new(25. * scale, 12., x * scale + 0.25 * scale),
        ));
    }
    for x in [-12., 0., 12.] {
        for z in [-8., 8.] {
            colliders.push(Aabb::new(
                Vec3::new((x - 1.5) * scale, 0., (z - 1.5) * scale),
                Vec3::new((x + 1.5) * scale, 5., (z + 1.5) * scale),
            ));
        }
    }
    for index in 0..STAIR_COUNT {
        let height = STAIR_RISE * (index + 1) as f32;
        let z = ROUTE_LOW_Z - (index as f32 + 0.5) * STAIR_TREAD;
        colliders.push(Aabb::new(
            Vec3::new(
                (STAIR_X - STAIR_WIDTH * 0.5) * scale,
                0.0,
                (z - STAIR_TREAD * 0.5) * scale,
            ),
            Vec3::new(
                (STAIR_X + STAIR_WIDTH * 0.5) * scale,
                height,
                (z + STAIR_TREAD * 0.5) * scale,
            ),
        ));
    }
    let stair_landing_start = ROUTE_LOW_Z - STAIR_COUNT as f32 * STAIR_TREAD;
    colliders.push(Aabb::new(
        Vec3::new(
            (STAIR_X - STAIR_WIDTH * 0.5) * scale,
            0.0,
            (stair_landing_start - 3.0) * scale,
        ),
        Vec3::new(
            (STAIR_X + STAIR_WIDTH * 0.5) * scale,
            STAIR_COUNT as f32 * STAIR_RISE,
            stair_landing_start * scale,
        ),
    ));

    let segment_depth = RAMP_LENGTH / RAMP_SEGMENTS as f32;
    for index in 0..RAMP_SEGMENTS {
        let z_high = ROUTE_LOW_Z - index as f32 * segment_depth;
        let z_low = z_high - segment_depth;
        let height = RAMP_RISE * (index + 1) as f32 / RAMP_SEGMENTS as f32;
        colliders.push(Aabb::new(
            Vec3::new((RAMP_X - RAMP_WIDTH * 0.5) * scale, 0.0, z_low * scale),
            Vec3::new((RAMP_X + RAMP_WIDTH * 0.5) * scale, height, z_high * scale),
        ));
    }
    let ramp_high_z = ROUTE_LOW_Z - RAMP_LENGTH;
    colliders.push(Aabb::new(
        Vec3::new(
            (RAMP_X - RAMP_WIDTH * 0.5) * scale,
            0.0,
            (ramp_high_z - 2.0) * scale,
        ),
        Vec3::new(
            (RAMP_X + RAMP_WIDTH * 0.5) * scale,
            RAMP_RISE,
            ramp_high_z * scale,
        ),
    ));
    colliders
}
