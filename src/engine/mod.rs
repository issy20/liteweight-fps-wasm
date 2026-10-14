use glam::Vec3;

pub mod layout;
pub mod renderer;
pub mod text;
pub mod text_render;
pub mod window;

/// 壁面識別用の細い格子線。シェーダー(scene_wall.wgsl)と同一の計算をRustで再現する。
/// 水平面（法線が±Y）はワールドx/z、垂直面は高さyを含む格子を描く。
/// WGSLのfract()は常に[0,1)を返すため、負の座標でも同じ挙動に合わせる。
/// 戻り値は色の乗数（1.0=線なし、0.94=線上）。
pub fn grid_shade(world: Vec3, normal: Vec3, grid: f32, line_width: f32) -> f32 {
    let is_horizontal = normal.y.abs() > 0.5;
    let (cell_a, cell_b) = if is_horizontal {
        (
            (world.x / grid).fract().rem_euclid(1.0),
            (world.z / grid).fract().rem_euclid(1.0),
        )
    } else {
        (
            (world.x / grid).fract().rem_euclid(1.0),
            (world.y / grid).fract().rem_euclid(1.0),
        )
    };
    let near_a = cell_a.min(1.0 - cell_a);
    let near_b = cell_b.min(1.0 - cell_b);
    let d = near_a.min(near_b);
    if d < line_width {
        0.94
    } else {
        1.0
    }
}

pub mod audio;
