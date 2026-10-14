//! 低コストの3Dモデルと最小HUD。テクスチャ・フォントの依存は持たない。
pub mod menu;

use crate::engine::text::{FontAtlas, TextMesh};
use crate::{
    simulation::{
        combat::Combat,
        weapon::{Weapon, WeaponKind},
    },
    Scene, Vertex,
};
use glam::{Vec2, Vec3};

pub fn world_mesh(combat: &Combat) -> Scene {
    let mut mesh = Scene {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    world_mesh_into(&mut mesh, combat);
    mesh
}

pub fn world_mesh_into(mesh: &mut Scene, combat: &Combat) {
    mesh.vertices.clear();
    mesh.indices.clear();
    mesh.vertices
        .reserve(combat.targets.len() * BOT_VERTEX_CAPACITY + 24);
    mesh.indices
        .reserve(combat.targets.len() * BOT_INDEX_CAPACITY + 36);
    for t in combat.targets.iter().filter(|target| target.alive) {
        mannequin(mesh, t.position, t.yaw, t.flash > 0.0);
    }
    if combat.feedback > 0.0 && combat.last_shot.impact {
        mesh.cube(
            combat.last_shot.position,
            Vec3::splat(0.025),
            [1., 0.9, 0.4],
        );
    }
}

const BOT_VERTEX_CAPACITY: usize = 256;
const BOT_INDEX_CAPACITY: usize = 960;

fn mannequin(mesh: &mut Scene, position: Vec3, yaw: f32, hit_flash: bool) {
    let first_vertex = mesh.vertices.len();
    let color = if hit_flash {
        [1.0, 0.95, 0.72]
    } else {
        [0.74, 0.69, 0.58]
    };

    // Build the trunk as one shaped surface so the chest, waist, and hips read as anatomy.
    let torso = [
        (0.96, 0.16, 0.12),
        (1.06, 0.19, 0.135),
        (1.17, 0.14, 0.112),
        (1.38, 0.21, 0.15),
        (1.50, 0.24, 0.14),
        (1.67, 0.16, 0.10),
    ];
    loft_torso(mesh, position, &torso, color);

    for side in [-1.0, 1.0] {
        let shoulder = position + Vec3::new(side * 0.20, 1.52, 0.0);
        let elbow = position + Vec3::new(side * 0.31, 1.19, 0.02);
        let wrist = position + Vec3::new(side * 0.34, 0.77, 0.04);
        ringed_limb(
            mesh,
            &[
                (shoulder, 0.078, 0.09),
                (elbow, 0.056, 0.067),
                (wrist, 0.041, 0.05),
            ],
            color,
        );
        ellipsoid(
            mesh,
            position + Vec3::new(side * 0.345, 0.72, 0.045),
            Vec3::new(0.042, 0.065, 0.042),
            color,
        );

        let thigh = position + Vec3::new(side * 0.105, 0.96, 0.0);
        let knee = position + Vec3::new(side * 0.12, 0.50, 0.015);
        let ankle = position + Vec3::new(side * 0.14, 0.12, 0.015);
        ringed_limb(
            mesh,
            &[
                (thigh, 0.087, 0.11),
                (knee, 0.062, 0.075),
                (ankle, 0.045, 0.052),
            ],
            color,
        );
        ellipsoid(
            mesh,
            position + Vec3::new(side * 0.14, 0.045, 0.085),
            Vec3::new(0.075, 0.045, 0.17),
            color,
        );
    }

    let neck = position + Vec3::new(0.0, 1.65, 0.0);
    ringed_limb(
        mesh,
        &[
            (neck, 0.032, 0.032),
            (position + Vec3::new(0.0, 1.78, 0.0), 0.025, 0.025),
        ],
        color,
    );
    let head = position + Vec3::new(0.0, 1.90, 0.0);
    ellipsoid(mesh, head, Vec3::new(0.085, 0.13, 0.082), color);
    forehead_marker(mesh, head, [1.0, 0.48, 0.08]);

    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    for vertex in &mut mesh.vertices[first_vertex..] {
        let relative = Vec3::from_array(vertex.position) - position;
        vertex.position = (Vec3::new(
            relative.x * cos_yaw - relative.z * sin_yaw,
            relative.y,
            relative.x * sin_yaw + relative.z * cos_yaw,
        ) + position)
            .to_array();
    }
}

const ELLIPSOID_LONGITUDES: usize = 6;
const ELLIPSOID_LATITUDES: usize = 3;
const BODY_SIDES: usize = 6;
const RING_POINTS: [(f32, f32); ELLIPSOID_LONGITUDES] = [
    (1.0, 0.0),
    (0.5, 0.866_025_4),
    (-0.5, 0.866_025_4),
    (-1.0, 0.0),
    (-0.5, -0.866_025_4),
    (0.5, -0.866_025_4),
];

fn ellipsoid(mesh: &mut Scene, center: Vec3, radii: Vec3, color: [f32; 3]) {
    let base = mesh.vertices.len() as u16;
    mesh.vertices.push(Vertex {
        position: (center - Vec3::Y * radii.y).to_array(),
        color: [color[0] * 0.9, color[1] * 0.9, color[2] * 0.9, 1.0],
    });
    for latitude in 1..ELLIPSOID_LATITUDES {
        let angle = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::PI * latitude as f32 / ELLIPSOID_LATITUDES as f32;
        let ring_radius = angle.cos();
        let y = angle.sin() * radii.y;
        for (x, z) in RING_POINTS {
            let shade = 0.84 + 0.16 * (0.5 + 0.5 * z) * (0.96 + 0.04 * angle.cos());
            let point = center + Vec3::new(x * ring_radius * radii.x, y, z * ring_radius * radii.z);
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                color: [color[0] * shade, color[1] * shade, color[2] * shade, 1.0],
            });
        }
    }
    let top = base + (1 + (ELLIPSOID_LATITUDES - 1) * ELLIPSOID_LONGITUDES) as u16;
    mesh.vertices.push(Vertex {
        position: (center + Vec3::Y * radii.y).to_array(),
        color: [color[0] * 0.9, color[1] * 0.9, color[2] * 0.9, 1.0],
    });

    let first_ring = base + 1;
    for longitude in 0..ELLIPSOID_LONGITUDES {
        let next = (longitude + 1) % ELLIPSOID_LONGITUDES;
        let current = first_ring + longitude as u16;
        let adjacent = first_ring + next as u16;
        mesh.indices.extend([base, current, adjacent]);
    }
    for latitude in 0..(ELLIPSOID_LATITUDES - 2) {
        for longitude in 0..ELLIPSOID_LONGITUDES {
            let next = (longitude + 1) % ELLIPSOID_LONGITUDES;
            let lower = first_ring + (latitude * ELLIPSOID_LONGITUDES + longitude) as u16;
            let lower_next = first_ring + (latitude * ELLIPSOID_LONGITUDES + next) as u16;
            let upper = lower + ELLIPSOID_LONGITUDES as u16;
            let upper_next = lower_next + ELLIPSOID_LONGITUDES as u16;
            mesh.indices
                .extend([lower, lower_next, upper_next, lower, upper_next, upper]);
        }
    }
    let last_ring = first_ring + ((ELLIPSOID_LATITUDES - 2) * ELLIPSOID_LONGITUDES) as u16;
    for longitude in 0..ELLIPSOID_LONGITUDES {
        let next = (longitude + 1) % ELLIPSOID_LONGITUDES;
        mesh.indices
            .extend([top, last_ring + next as u16, last_ring + longitude as u16]);
    }
}

fn loft_torso(mesh: &mut Scene, position: Vec3, rings: &[(f32, f32, f32)], color: [f32; 3]) {
    let base = mesh.vertices.len() as u16;
    for (y, radius_x, radius_z) in rings.iter().copied() {
        for (x, z) in RING_POINTS {
            let point = position + Vec3::new(x * radius_x, y, z * radius_z);
            let shade = 0.86 + 0.14 * (0.5 + 0.5 * z);
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                color: [color[0] * shade, color[1] * shade, color[2] * shade, 1.0],
            });
        }
    }
    let bottom = mesh.vertices.len() as u16;
    mesh.vertices.push(Vertex {
        position: (position + Vec3::new(0.0, rings[0].0, 0.0)).to_array(),
        color: [color[0] * 0.9, color[1] * 0.9, color[2] * 0.9, 1.0],
    });
    let top = mesh.vertices.len() as u16;
    mesh.vertices.push(Vertex {
        position: (position + Vec3::new(0.0, rings[rings.len() - 1].0, 0.0)).to_array(),
        color: [color[0] * 0.9, color[1] * 0.9, color[2] * 0.9, 1.0],
    });
    for ring in 0..(rings.len() - 1) {
        for side in 0..BODY_SIDES {
            let next = (side + 1) % BODY_SIDES;
            let lower = base + (ring * BODY_SIDES + side) as u16;
            let lower_next = base + (ring * BODY_SIDES + next) as u16;
            let upper = lower + BODY_SIDES as u16;
            let upper_next = lower_next + BODY_SIDES as u16;
            mesh.indices
                .extend([lower, lower_next, upper_next, lower, upper_next, upper]);
        }
    }
    for side in 0..BODY_SIDES {
        let next = (side + 1) % BODY_SIDES;
        mesh.indices
            .extend([bottom, base + next as u16, base + side as u16]);
        let last_ring = base + ((rings.len() - 1) * BODY_SIDES) as u16;
        mesh.indices
            .extend([top, last_ring + side as u16, last_ring + next as u16]);
    }
}

fn ringed_limb(mesh: &mut Scene, rings: &[(Vec3, f32, f32)], color: [f32; 3]) {
    let base = mesh.vertices.len() as u16;
    for (center, radius_x, radius_z) in rings.iter().copied() {
        for (x, z) in RING_POINTS {
            let point = center + Vec3::new(x * radius_x, 0.0, z * radius_z);
            let shade = 0.88 + 0.12 * (0.5 + 0.5 * z);
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                color: [color[0] * shade, color[1] * shade, color[2] * shade, 1.0],
            });
        }
    }
    for ring in 0..(rings.len() - 1) {
        for side in 0..BODY_SIDES {
            let next = (side + 1) % BODY_SIDES;
            let lower = base + (ring * BODY_SIDES + side) as u16;
            let lower_next = base + (ring * BODY_SIDES + next) as u16;
            let upper = lower + BODY_SIDES as u16;
            let upper_next = lower_next + BODY_SIDES as u16;
            mesh.indices
                .extend([lower, lower_next, upper_next, lower, upper_next, upper]);
        }
    }
}

fn forehead_marker(mesh: &mut Scene, head_center: Vec3, color: [f32; 3]) {
    const HEAD_RADII: Vec3 = Vec3::new(0.085, 0.13, 0.082);
    const MARKER_Y: f32 = 0.035;
    const MARKER_RADIUS_X: f32 = 0.032;
    const MARKER_RADIUS_Y: f32 = 0.023;
    let marker_position = |x: f32, y: f32| {
        let local_y = MARKER_Y + y;
        let z = HEAD_RADII.z
            * (1.0 - (x / HEAD_RADII.x).powi(2) - (local_y / HEAD_RADII.y).powi(2)).sqrt()
            + 0.001;
        head_center + Vec3::new(x, local_y, z)
    };
    let base = mesh.vertices.len() as u16;
    mesh.vertices.push(Vertex {
        position: marker_position(0.0, 0.0).to_array(),
        color: [color[0], color[1], color[2], 1.0],
    });
    for (x, y) in [
        (MARKER_RADIUS_X, 0.0),
        (MARKER_RADIUS_X * 0.5, MARKER_RADIUS_Y * 0.866_025_4),
        (-MARKER_RADIUS_X * 0.5, MARKER_RADIUS_Y * 0.866_025_4),
        (-MARKER_RADIUS_X, 0.0),
        (-MARKER_RADIUS_X * 0.5, -MARKER_RADIUS_Y * 0.866_025_4),
        (MARKER_RADIUS_X * 0.5, -MARKER_RADIUS_Y * 0.866_025_4),
    ] {
        mesh.vertices.push(Vertex {
            position: marker_position(x, y).to_array(),
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    for index in 0..ELLIPSOID_LONGITUDES {
        let next = (index + 1) % ELLIPSOID_LONGITUDES;
        mesh.indices
            .extend([base, base + 1 + index as u16, base + 1 + next as u16]);
    }
}

fn rect(mesh: &mut Scene, x: f32, y: f32, w: f32, h: f32, color: [f32; 3], aspect: f32) {
    rect_alpha(
        mesh,
        x,
        y,
        w,
        h,
        [color[0], color[1], color[2], 1.0],
        aspect,
    );
}

fn stroke_segment(
    mesh: &mut Scene,
    start: [f32; 2],
    end: [f32; 2],
    thickness: f32,
    color: [f32; 3],
    aspect: f32,
) {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
    if length <= f32::EPSILON {
        return;
    }
    let normal = [
        -delta[1] / length * thickness * 0.5,
        delta[0] / length * thickness * 0.5,
    ];
    let base = mesh.vertices.len() as u16;
    for point in [
        [start[0] + normal[0], start[1] + normal[1]],
        [end[0] + normal[0], end[1] + normal[1]],
        [end[0] - normal[0], end[1] - normal[1]],
        [start[0] - normal[0], start[1] - normal[1]],
    ] {
        mesh.vertices.push(Vertex {
            position: [point[0] / aspect, point[1], 0.0],
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn triangle(
    mesh: &mut Scene,
    tip: [f32; 2],
    left: [f32; 2],
    right: [f32; 2],
    color: [f32; 3],
    aspect: f32,
) {
    let base = mesh.vertices.len() as u16;
    for point in [tip, left, right] {
        mesh.vertices.push(Vertex {
            position: [point[0] / aspect, point[1], 0.0],
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    mesh.indices.extend([base, base + 1, base + 2]);
}

fn slanted_rect(mesh: &mut Scene, shape: [f32; 6], color: [f32; 3], aspect: f32) {
    let [bottom_x, y, bottom_width, top_width, height, top_shift] = shape;
    let base = mesh.vertices.len() as u16;
    for (x, y) in [
        (bottom_x - bottom_width * 0.5, y),
        (bottom_x + bottom_width * 0.5, y),
        (bottom_x + top_shift + top_width * 0.5, y + height),
        (bottom_x + top_shift - top_width * 0.5, y + height),
    ] {
        mesh.vertices.push(Vertex {
            position: [x / aspect, y, 0.0],
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn rect_alpha(mesh: &mut Scene, x: f32, y: f32, w: f32, h: f32, color: [f32; 4], aspect: f32) {
    let base = mesh.vertices.len() as u16;
    for (dx, dy) in [(0., 0.), (w, 0.), (w, h), (0., h)] {
        mesh.vertices.push(Vertex {
            position: [(x + dx) / aspect, y + dy, 0.],
            color,
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn scope_model_ring(mesh: &mut Scene, center: Vec2, aspect: f32, progress: f32, opacity: f32) {
    const SEGMENTS: usize = 12;
    const INITIAL_OUTER_RADIUS: f32 = 0.05;
    const INITIAL_INNER_RADIUS: f32 = 0.035;
    let progress = progress.clamp(0.0, 1.0);
    let eased = progress * progress * (3.0 - 2.0 * progress);
    let outer_radius = INITIAL_OUTER_RADIUS + (SCOPE_MAX_RADIUS - INITIAL_OUTER_RADIUS) * eased;
    let inner_radius = INITIAL_INNER_RADIUS + (outer_radius - INITIAL_INNER_RADIUS) * eased;
    let color = [0.18, 0.21, 0.23, opacity];
    for segment in 0..SEGMENTS {
        let angle0 = segment as f32 * std::f32::consts::TAU / SEGMENTS as f32;
        let angle1 = (segment + 1) as f32 * std::f32::consts::TAU / SEGMENTS as f32;
        let base = mesh.vertices.len() as u16;
        for (angle, radius) in [
            (angle0, outer_radius),
            (angle0, inner_radius),
            (angle1, inner_radius),
            (angle1, outer_radius),
        ] {
            mesh.vertices.push(Vertex {
                position: [
                    (center.x + angle.cos() * radius) / aspect,
                    center.y + angle.sin() * radius,
                    0.0,
                ],
                color,
            });
        }
        mesh.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

const SETTINGS_VISIBLE_ROWS: usize = 11;
const SETTINGS_ROW_TOP: f32 = 0.49;
const SETTINGS_ROW_GAP: f32 = 0.095;

fn settings_visible_range(selected: usize, total: usize) -> std::ops::Range<usize> {
    if total <= SETTINGS_VISIBLE_ROWS {
        return 0..total;
    }
    let start = selected
        .saturating_sub(SETTINGS_VISIBLE_ROWS / 2)
        .min(total - SETTINGS_VISIBLE_ROWS);
    start..start + SETTINGS_VISIBLE_ROWS
}

pub fn settings_overlay(
    mesh: &mut Scene,
    settings: &crate::persistence::GameSettings,
    selected: usize,
    aspect: f32,
) {
    // Fade the live scene so the panel stays readable without hiding the game entirely.
    rect_alpha(
        mesh,
        -aspect,
        -1.0,
        2.0 * aspect,
        2.0,
        [0.008, 0.012, 0.018, 0.68],
        aspect,
    );

    // Centered settings card with a clear edge and warm focus accent.
    rect_alpha(
        mesh,
        -0.61 * aspect,
        -0.91,
        1.22 * aspect,
        1.82,
        [0.025, 0.036, 0.047, 0.98],
        aspect,
    );
    rect(
        mesh,
        -0.61 * aspect,
        -0.91,
        0.008 * aspect,
        1.82,
        [0.95, 0.53, 0.19],
        aspect,
    );

    let visible = settings_visible_range(selected, 24);
    for (row, setting_index) in visible.clone().enumerate() {
        let center = SETTINGS_ROW_TOP - row as f32 * SETTINGS_ROW_GAP - 0.02;
        let height = 0.070;
        let bottom = center - height * 0.5;
        let selected_row = selected == setting_index;
        rect_alpha(
            mesh,
            -0.57 * aspect,
            bottom,
            1.14 * aspect,
            height,
            if selected_row {
                [0.92, 0.48, 0.16, 0.22]
            } else {
                [0.55, 0.64, 0.70, 0.045]
            },
            aspect,
        );
        if selected_row {
            rect(
                mesh,
                -0.57 * aspect,
                bottom,
                0.008 * aspect,
                height,
                [1.0, 0.62, 0.25],
                aspect,
            );
        }
    }

    for (setting_index, value) in [
        (1, settings.mouse_sensitivity / 5.0),
        (2, settings.ads_sensitivity),
        (10, settings.volume),
        (11, settings.hit_marker_strength),
        (13, (settings.crosshair_size - 0.5) / 1.5),
        (14, settings.crosshair_gap / 2.0),
        (15, (settings.crosshair_thickness - 0.5) / 1.5),
    ] {
        let Some(row) = visible.clone().position(|index| index == setting_index) else {
            continue;
        };
        let value = value.clamp(0.0, 1.0);
        let color = if selected == setting_index {
            [1.0, 0.65, 0.24]
        } else {
            [0.38, 0.61, 0.68]
        };
        let y = SETTINGS_ROW_TOP - row as f32 * SETTINGS_ROW_GAP - 0.025;
        rect(
            mesh,
            0.0,
            y,
            0.22 * aspect,
            0.006,
            [0.12, 0.17, 0.20],
            aspect,
        );
        rect(mesh, 0.0, y, 0.22 * aspect * value, 0.006, color, aspect);
        let knob_x = 0.22 * value;
        rect(
            mesh,
            (knob_x - 0.006) * aspect,
            y - 0.008,
            0.012 * aspect,
            0.022,
            [0.94, 0.95, 0.92],
            aspect,
        );
    }
}

pub fn settings_text(
    settings: &crate::persistence::GameSettings,
    selected: usize,
    viewport: [u32; 2],
) -> TextMesh {
    settings_text_for_context(settings, selected, viewport, false)
}

pub fn settings_text_for_context(
    settings: &crate::persistence::GameSettings,
    selected: usize,
    viewport: [u32; 2],
    from_title: bool,
) -> TextMesh {
    settings_text_with_bindings(settings, selected, viewport, from_title, 0, false, 0)
}

#[allow(clippy::too_many_arguments)]
pub fn settings_text_with_bindings(
    settings: &crate::persistence::GameSettings,
    selected: usize,
    viewport: [u32; 2],
    from_title: bool,
    binding_selection: usize,
    waiting_for_binding: bool,
    profile: u8,
) -> TextMesh {
    let mut mesh = TextMesh::default();
    settings_text_with_bindings_into(
        &mut mesh,
        settings,
        selected,
        viewport,
        from_title,
        binding_selection,
        waiting_for_binding,
        profile,
    );
    mesh
}

#[allow(clippy::too_many_arguments)]
pub fn settings_text_with_bindings_into(
    mesh: &mut TextMesh,
    settings: &crate::persistence::GameSettings,
    selected: usize,
    viewport: [u32; 2],
    from_title: bool,
    binding_selection: usize,
    waiting_for_binding: bool,
    profile: u8,
) {
    use crate::persistence::BindingAction;

    mesh.clear();
    let [width, height] = viewport;
    let font = FontAtlas::shared();
    let mut line = |text: &str, x_ndc: f32, y_ndc: f32, size: f32, color: [f32; 4]| {
        let font_px = height as f32 * size;
        let measured = font.measure_width(text, font_px);
        let x_px = (x_ndc + 1.0) * width as f32 * 0.5;
        font.layout_into(
            mesh,
            text,
            [x_px, (1.0 - y_ndc) * height as f32 * 0.5],
            font_px,
            viewport,
            color,
        );
        measured
    };
    let normal = [0.82, 0.87, 0.89, 1.0];
    let active = [1.0, 0.65, 0.24, 1.0];
    let warning = [1.0, 0.49, 0.32, 1.0];
    let _ = line(
        if from_title { "SETTINGS" } else { "PAUSED" },
        -0.08,
        0.80,
        0.064,
        [0.98, 0.72, 0.39, 1.0],
    );
    let _ = line(
        if from_title {
            "GAMEPLAY OPTIONS"
        } else {
            "TRAINING PAUSED"
        },
        -0.23,
        0.69,
        0.026,
        [0.59, 0.68, 0.71, 1.0],
    );

    let binding_action = BindingAction::ALL[binding_selection % BindingAction::ALL.len()];
    let fps_label = settings
        .fps_limit()
        .map_or_else(|| "OFF".to_owned(), |fps| format!("{fps} FPS"));
    let behavior = match settings.bot_behavior {
        0 => "STATIC",
        1 => "STRAFE",
        2 => "CHASE",
        _ => "MIXED",
    };
    let mut rows = vec![
        (
            if from_title {
                "BACK TO MENU".to_owned()
            } else {
                "RESUME".to_owned()
            },
            String::new(),
        ),
        (
            "MOUSE SENSITIVITY".to_owned(),
            format!("{:.2}", settings.mouse_sensitivity),
        ),
        (
            "ADS SENSITIVITY".to_owned(),
            format!("{:.2}", settings.ads_sensitivity),
        ),
        (
            "FIELD OF VIEW".to_owned(),
            format!("{:.0} DEG", settings.fov),
        ),
        (
            "INVERT Y AXIS".to_owned(),
            if settings.invert_y { "ON" } else { "OFF" }.to_owned(),
        ),
        (
            "KEY BINDING".to_owned(),
            format!(
                "{}  {}",
                binding_action.label(),
                settings.key_bindings.get(binding_action).label()
            ),
        ),
        (
            "INTERNAL RESOLUTION".to_owned(),
            format!("{}%", (settings.render_scale() * 100.0).round()),
        ),
        (
            "FULLSCREEN".to_owned(),
            if settings.fullscreen { "ON" } else { "OFF" }.to_owned(),
        ),
        ("FPS LIMIT".to_owned(), fps_label),
        (
            "INFINITE AMMO".to_owned(),
            if settings.infinite_ammo { "ON" } else { "OFF" }.to_owned(),
        ),
        (
            "SOUND VOLUME".to_owned(),
            format!("{:.0}%", settings.volume * 100.0),
        ),
        (
            "HIT MARKER STRENGTH".to_owned(),
            format!("{:.0}%", settings.hit_marker_strength * 100.0),
        ),
        (
            "CROSSHAIR COLOR".to_owned(),
            crate::persistence::CROSSHAIR_COLORS[usize::from(settings.crosshair_color.min(5))]
                .to_owned(),
        ),
        (
            "CROSSHAIR SIZE".to_owned(),
            format!("{:.1}X", settings.crosshair_size),
        ),
        (
            "CROSSHAIR GAP".to_owned(),
            format!("{:.1}X", settings.crosshair_gap),
        ),
        (
            "CROSSHAIR THICKNESS".to_owned(),
            format!("{:.1}X", settings.crosshair_thickness),
        ),
        (
            "CROSSHAIR DOT".to_owned(),
            if settings.crosshair_dot { "ON" } else { "OFF" }.to_owned(),
        ),
        ("BOT COUNT".to_owned(), settings.bot_count.to_string()),
        ("BOT BEHAVIOR".to_owned(), behavior.to_owned()),
        ("BOT HP".to_owned(), settings.bot_hp.to_string()),
        (
            "BOT SPEED".to_owned(),
            format!("{:.1}X", settings.bot_speed),
        ),
        (
            "BOT RESPAWN".to_owned(),
            format!("{:.2}S", settings.bot_respawn_seconds),
        ),
        (
            "PROFILE".to_owned(),
            format!("{}/{}", profile + 1, crate::persistence::PROFILE_COUNT),
        ),
        (
            if from_title {
                "BACK TO MENU"
            } else {
                "CHANGE MODE"
            }
            .to_owned(),
            String::new(),
        ),
    ];
    if waiting_for_binding {
        rows[5].1 = "PRESS A KEY  (ESC CANCEL)".to_owned();
    }
    let visible = settings_visible_range(selected, rows.len());
    for (row, index) in visible.clone().enumerate() {
        let (label, value) = &rows[index];
        let y = SETTINGS_ROW_TOP - row as f32 * SETTINGS_ROW_GAP;
        let color = if selected == index { active } else { normal };
        let _ = line(label, -0.55, y, 0.024, color);
        if !value.is_empty() {
            let _ = line(
                value,
                0.30,
                y,
                0.024,
                if waiting_for_binding && index == 5 {
                    warning
                } else {
                    color
                },
            );
        }
    }
    let _ = line(
        "UP/DOWN SELECT   LEFT/RIGHT ADJUST   ENTER REBIND   HOME DEFAULTS",
        -0.55,
        -0.86,
        0.018,
        normal,
    );
    let _ = line(
        &format!(
            "OPTIONS {}-{} / {}",
            visible.start + 1,
            visible.end,
            rows.len()
        ),
        0.37,
        0.69,
        0.018,
        normal,
    );
}

pub fn title_overlay(mesh: &mut Scene, selected: usize, aspect: f32) {
    modal_backdrop(mesh, aspect);
    rect_alpha(
        mesh,
        -0.53 * aspect,
        -0.88,
        1.06 * aspect,
        1.72,
        [0.025, 0.036, 0.047, 0.98],
        aspect,
    );
    rect(
        mesh,
        -0.53 * aspect,
        -0.88,
        0.008 * aspect,
        1.72,
        [0.95, 0.53, 0.19],
        aspect,
    );
    for (index, (bottom, height)) in [
        (0.085, 0.19),
        (-0.135, 0.19),
        (-0.355, 0.19),
        (-0.575, 0.19),
    ]
    .into_iter()
    .enumerate()
    {
        let is_selected = index == selected;
        rect_alpha(
            mesh,
            -0.45 * aspect,
            bottom,
            0.90 * aspect,
            height,
            if is_selected {
                [0.92, 0.48, 0.16, 0.22]
            } else {
                [0.55, 0.64, 0.70, 0.045]
            },
            aspect,
        );
        if is_selected {
            rect(
                mesh,
                -0.45 * aspect,
                bottom,
                0.008 * aspect,
                height,
                [1.0, 0.62, 0.25],
                aspect,
            );
        }
    }
}

fn append_centered_text_line(
    mesh: &mut TextMesh,
    font: &FontAtlas,
    text: &str,
    y_ndc: f32,
    size: f32,
    viewport: [u32; 2],
    color: [f32; 4],
) {
    let font_px = viewport[1] as f32 * size;
    let width = font.measure_width(text, font_px);
    let x = (viewport[0] as f32 - width) * 0.5;
    font.layout_into(
        mesh,
        text,
        [x, (1.0 - y_ndc) * viewport[1] as f32 * 0.5],
        font_px,
        viewport,
        color,
    );
}

/// Draws a centered line whose glyph bounds are vertically centered on
/// `center_y_ndc`, so labels sit in the middle of their background band.
fn append_vertically_centered_text_line(
    mesh: &mut TextMesh,
    font: &FontAtlas,
    text: &str,
    center_y_ndc: f32,
    size: f32,
    viewport: [u32; 2],
    color: [f32; 4],
) {
    let font_px = viewport[1] as f32 * size;
    let width = font.measure_width(text, font_px);
    let x = (viewport[0] as f32 - width) * 0.5;
    let start = mesh.vertices.len();
    font.layout_into(
        mesh,
        text,
        [x, (1.0 - center_y_ndc) * viewport[1] as f32 * 0.5],
        font_px,
        viewport,
        color,
    );
    if mesh.vertices.len() == start {
        return;
    }
    let (min_y, max_y) = mesh.vertices[start..].iter().fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(min_y, max_y), vertex| (min_y.min(vertex.position[1]), max_y.max(vertex.position[1])),
    );
    let shift = center_y_ndc - (min_y + max_y) * 0.5;
    for vertex in &mut mesh.vertices[start..] {
        vertex.position[1] += shift;
    }
}

pub fn title_text(selected: usize, viewport: [u32; 2]) -> TextMesh {
    let mut mesh = TextMesh::default();
    title_text_into(&mut mesh, selected, viewport);
    mesh
}

pub fn title_text_into(mesh: &mut TextMesh, selected: usize, viewport: [u32; 2]) {
    mesh.clear();
    let font = FontAtlas::shared();
    append_centered_text_line(
        mesh,
        font,
        "LITEWEIGHT FPS",
        0.62,
        0.075,
        viewport,
        [0.98, 0.72, 0.39, 1.0],
    );
    append_centered_text_line(
        mesh,
        font,
        "FULL FPS. MINIMAL GRAPHICS.",
        0.46,
        0.029,
        viewport,
        [0.59, 0.68, 0.71, 1.0],
    );
    for (index, (label, center)) in [
        ("FREE TRAINING", 0.18_f32),
        ("TRACKING AIM", -0.04),
        ("SETTINGS", -0.26),
        ("CREDITS", -0.48),
    ]
    .into_iter()
    .enumerate()
    {
        let color = if selected == index {
            [1.0, 0.65, 0.24, 1.0]
        } else {
            [0.82, 0.87, 0.89, 1.0]
        };
        append_vertically_centered_text_line(mesh, font, label, center, 0.047, viewport, color);
    }
    append_centered_text_line(
        mesh,
        font,
        "UP / DOWN MOVE      ENTER SELECT",
        -0.66,
        0.027,
        viewport,
        [0.67, 0.75, 0.77, 1.0],
    );
    append_centered_text_line(
        mesh,
        font,
        &format!("v{}", crate::VERSION),
        -0.745,
        0.020,
        viewport,
        [0.45, 0.52, 0.56, 1.0],
    );
    append_centered_text_line(
        mesh,
        font,
        "(C) 2026 ISSY20   FONT: CANTARELL (OFL)",
        -0.815,
        0.017,
        viewport,
        [0.42, 0.48, 0.52, 1.0],
    );
}

/// Dim backdrop and panel shown while the pointer is not captured.
pub fn capture_prompt_overlay(mesh: &mut Scene, aspect: f32) {
    modal_backdrop(mesh, aspect);
    rect_alpha(
        mesh,
        -0.40 * aspect,
        -0.16,
        0.80 * aspect,
        0.32,
        [0.030, 0.040, 0.055, 0.96],
        aspect,
    );
    rect(
        mesh,
        -0.40 * aspect,
        -0.16,
        0.006 * aspect,
        0.32,
        [0.95, 0.53, 0.19],
        aspect,
    );
}

/// Prompt text for [`capture_prompt_overlay`].
pub fn capture_prompt_text(mesh: &mut TextMesh, title: &str, subtitle: &str, viewport: [u32; 2]) {
    let font = FontAtlas::shared();
    append_vertically_centered_text_line(
        mesh,
        font,
        title,
        0.055,
        0.05,
        viewport,
        [1.0, 0.72, 0.39, 1.0],
    );
    append_vertically_centered_text_line(
        mesh,
        font,
        subtitle,
        -0.055,
        0.022,
        viewport,
        [0.75, 0.81, 0.83, 1.0],
    );
}

/// Dim backdrop and panel for the credits screen.
pub fn credits_overlay(mesh: &mut Scene, aspect: f32) {
    modal_backdrop(mesh, aspect);
    rect_alpha(
        mesh,
        -0.52 * aspect,
        -0.38,
        1.04 * aspect,
        0.68,
        [0.030, 0.040, 0.055, 0.97],
        aspect,
    );
    rect(
        mesh,
        -0.52 * aspect,
        -0.38,
        0.006 * aspect,
        0.68,
        [0.95, 0.53, 0.19],
        aspect,
    );
}

/// Text for the credits screen.
pub fn credits_text(mesh: &mut TextMesh, viewport: [u32; 2]) {
    mesh.clear();
    let font = FontAtlas::shared();
    append_vertically_centered_text_line(
        mesh,
        font,
        "CREDITS",
        0.22,
        0.048,
        viewport,
        [0.98, 0.72, 0.39, 1.0],
    );
    let rows = [
        ("GAME", "LITEWEIGHT-FPS  MIT OR APACHE-2.0"),
        ("ORIGIN", "PORT OF LITEWEIGHT-FPS BY ISSY20"),
        ("FONT", "CANTARELL  SIL OPEN FONT LICENSE"),
        ("ENGINE", "WGPU / WINIT / GLAM / AB_GLYPH / SERDE"),
        ("PRIVACY", "SETTINGS & RECORDS STORED LOCALLY. NO TRACKING."),
        ("LICENSES", "LICENSE-MIT / LICENSE-APACHE / OFL.TXT"),
    ];
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let y = 0.11 - index as f32 * 0.074;
        append_vertically_centered_text_line(
            mesh,
            font,
            &format!("{label}   {value}"),
            y,
            0.026,
            viewport,
            [0.85, 0.89, 0.90, 1.0],
        );
    }
    append_vertically_centered_text_line(
        mesh,
        font,
        "ESC CLOSE      ENTER OPEN CREDITS PAGE",
        -0.31,
        0.024,
        viewport,
        [0.67, 0.75, 0.77, 1.0],
    );
    append_vertically_centered_text_line(
        mesh,
        font,
        &format!("v{}", crate::VERSION),
        -0.352,
        0.020,
        viewport,
        [0.45, 0.52, 0.56, 1.0],
    );
}

fn modal_backdrop(mesh: &mut Scene, aspect: f32) {
    rect_alpha(
        mesh,
        -aspect,
        -1.0,
        2.0 * aspect,
        2.0,
        [0.008, 0.012, 0.018, 0.68],
        aspect,
    );
}

pub struct GameHudInfo {
    pub mode: crate::simulation::game_mode::SessionMode,
    pub weapon: WeaponKind,
    pub ammo: u32,
    pub magazine: u32,
    pub kills: u32,
    pub accuracy: f32,
    pub headshot_rate: f32,
    pub infinite_ammo: bool,
    pub key_bindings: crate::persistence::KeyBindings,
}

pub fn game_hud_text(info: GameHudInfo, viewport: [u32; 2]) -> TextMesh {
    let mut mesh = TextMesh::default();
    game_hud_text_into(&mut mesh, info, viewport);
    mesh
}

pub fn game_hud_text_into(mesh: &mut TextMesh, info: GameHudInfo, viewport: [u32; 2]) {
    mesh.clear();
    let font = FontAtlas::shared();
    let mut line = |text: &str, y_ndc: f32, size: f32, color: [f32; 4]| {
        append_centered_text_line(mesh, font, text, y_ndc, size, viewport, color);
    };
    let muted = [0.79, 0.84, 0.84, 1.0];
    let status = match info.mode {
        crate::simulation::game_mode::SessionMode::FreeTraining => "FREE TRAINING",
        crate::simulation::game_mode::SessionMode::TrackingAim => "TRACKING AIM",
    };
    let ammo_text = if info.infinite_ammo {
        "∞".to_owned()
    } else {
        format!("{}/{}", info.ammo, info.magazine)
    };
    line(status, 0.97, 0.034, [0.98, 0.72, 0.39, 1.0]);
    line(
        &format!(
            "{}   AMMO {}   KILLS {}   ACC {:.0}%   HS {:.0}%",
            info.weapon.label(),
            ammo_text,
            info.kills,
            info.accuracy.clamp(0.0, 1.0) * 100.0,
            info.headshot_rate.clamp(0.0, 1.0) * 100.0,
        ),
        0.87,
        0.026,
        muted,
    );
    let bindings = info.key_bindings;
    line(
        &format!(
            "{}{}{}{} MOVE   {} DASH   {} JUMP",
            bindings.forward.label(),
            bindings.left.label(),
            bindings.backward.label(),
            bindings.right.label(),
            bindings.dash.label(),
            bindings.jump.label(),
        ),
        -0.82,
        0.023,
        muted,
    );
    line(
        &format!(
            "LMB FIRE   RMB ADS   {} RELOAD   {}/{}/{} WEAPON   {} PAUSE",
            bindings.reload.label(),
            bindings.pistol.label(),
            bindings.assault_rifle.label(),
            bindings.sniper.label(),
            bindings.pause.label(),
        ),
        -0.90,
        0.023,
        muted,
    );
}

pub fn append_notice_text(mesh: &mut TextMesh, message: &str, viewport: [u32; 2], y_ndc: f32) {
    if message.is_empty() || viewport[0] == 0 || viewport[1] == 0 {
        return;
    }
    let font = FontAtlas::shared();
    let font_px = viewport[1] as f32 * 0.032;
    let measured = font.measure_width(message, font_px);
    font.layout_into(
        mesh,
        message,
        [
            ((viewport[0] as f32 - measured) * 0.5).max(8.0),
            (1.0 - y_ndc) * viewport[1] as f32 * 0.5,
        ],
        font_px,
        viewport,
        [1.0, 0.46, 0.26, 1.0],
    );
}

/// Session summary shown after leaving a run.
pub struct ResultInfo {
    pub mode: crate::simulation::game_mode::SessionMode,
    pub kills: u32,
    pub accuracy: f32,
    pub headshot_rate: f32,
    pub seconds: f32,
    pub new_best: bool,
}

/// Dim backdrop and panel for the session result screen.
pub fn result_overlay(mesh: &mut Scene, aspect: f32) {
    modal_backdrop(mesh, aspect);
    rect_alpha(
        mesh,
        -0.42 * aspect,
        -0.36,
        0.84 * aspect,
        0.66,
        [0.030, 0.040, 0.055, 0.97],
        aspect,
    );
    rect(
        mesh,
        -0.42 * aspect,
        -0.36,
        0.006 * aspect,
        0.66,
        [0.95, 0.53, 0.19],
        aspect,
    );
}

/// Text for [`result_overlay`].
pub fn result_text(mesh: &mut TextMesh, info: &ResultInfo, viewport: [u32; 2]) {
    mesh.clear();
    let font = FontAtlas::shared();
    let mode = match info.mode {
        crate::simulation::game_mode::SessionMode::FreeTraining => "FREE TRAINING",
        crate::simulation::game_mode::SessionMode::TrackingAim => "TRACKING AIM",
    };
    let minutes = (info.seconds.max(0.0) / 60.0).floor() as u32;
    let seconds = info.seconds.max(0.0) - minutes as f32 * 60.0;
    append_vertically_centered_text_line(
        mesh,
        font,
        "SESSION RESULT",
        0.25,
        0.026,
        viewport,
        [0.59, 0.68, 0.71, 1.0],
    );
    append_vertically_centered_text_line(
        mesh,
        font,
        mode,
        0.17,
        0.042,
        viewport,
        [0.98, 0.72, 0.39, 1.0],
    );
    let rows = [
        ("KILLS", format!("{}", info.kills)),
        (
            "ACCURACY",
            format!("{:.0}%", info.accuracy.clamp(0.0, 1.0) * 100.0),
        ),
        (
            "HEADSHOTS",
            format!("{:.0}%", info.headshot_rate.clamp(0.0, 1.0) * 100.0),
        ),
        ("TIME", format!("{minutes}:{seconds:04.1}")),
    ];
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let y = 0.06 - index as f32 * 0.070;
        append_vertically_centered_text_line(
            mesh,
            font,
            &format!("{label}   {value}"),
            y,
            0.028,
            viewport,
            [0.85, 0.89, 0.90, 1.0],
        );
    }
    if info.new_best {
        append_vertically_centered_text_line(
            mesh,
            font,
            "NEW BEST",
            -0.235,
            0.032,
            viewport,
            [1.0, 0.65, 0.24, 1.0],
        );
    }
    append_vertically_centered_text_line(
        mesh,
        font,
        "PRESS ENTER FOR TITLE",
        -0.31,
        0.020,
        viewport,
        [0.67, 0.75, 0.77, 1.0],
    );
}

fn digit(mesh: &mut Scene, n: u32, x: f32, y: f32, aspect: f32) {
    let masks = [
        0b1111110, 0b0110000, 0b1101101, 0b1111001, 0b0110011, 0b1011011, 0b1011111, 0b1110000,
        0b1111111, 0b1111011,
    ];
    let segments = [
        (0., 0.08, 0.04, 0.008),
        (0.032, 0.04, 0.008, 0.04),
        (0.032, 0., 0.008, 0.04),
        (0., 0., 0.04, 0.008),
        (0., 0., 0.008, 0.04),
        (0., 0.04, 0.008, 0.04),
        (0., 0.04, 0.04, 0.008),
    ];
    for (i, (dx, dy, w, h)) in segments.into_iter().enumerate() {
        if masks[n as usize] & (1 << (6 - i)) != 0 {
            rect(mesh, x + dx, y + dy, w, h, [0.9; 3], aspect);
        }
    }
}
const SCOPE_MAX_RADIUS: f32 = 1.90;
const SCOPE_MAX_SHADOW_WIDTH: f32 = 0.8;
const SCOPE_SHADOW_SEGMENTS: usize = 24;
const SCOPE_RING_SEGMENTS: usize = 32;
const SCOPE_TICK_SIZE_RATIO: f32 = 0.018;
const SCOPE_RETICLE_ARM: f32 = 0.07;
const SCOPE_RETICLE_LINE_RATIO: f32 = 0.0015;
const SCOPE_RETICLE_APPEAR_START_PROGRESS: f32 = 0.82;
const SCOPE_REFLECTION_SEGMENTS: usize = 16;
const SCOPE_REFLECTION_MAX_OPACITY: f32 = 0.055;
const SCOPE_GLASS_HAZE_SEGMENTS: usize = 32;
const SCOPE_GLASS_HAZE_MAX_OPACITY: f32 = 1.0;
const SCOPE_GLASS_HAZE_MIN_RADIUS: f32 = 0.055;
const SCOPE_GLASS_CLEAR_START_PROGRESS: f32 = 0.18;
const SCOPE_GLASS_CLEAR_END_PROGRESS: f32 = 0.82;
const SCOPE_GLASS_HAZE_CENTER_COLOR: [f32; 3] = [0.98, 0.96, 0.88];
const SCOPE_GLASS_HAZE_EDGE_COLOR: [f32; 3] = [0.68, 0.82, 0.90];
const PERIPHERAL_DARKEN_START: f32 = 0.3;
const PERIPHERAL_DARKEN_CURVE: f32 = 1.5;
const PERIPHERAL_MAX_OPACITY: f32 = 0.98;
const SCOPE_SHADOW_PEAK_ALPHA: f32 = 0.65;

fn scope_reticle_opacity(progress: f32) -> f32 {
    let fade = ((progress.clamp(0.0, 1.0) - SCOPE_RETICLE_APPEAR_START_PROGRESS)
        / (1.0 - SCOPE_RETICLE_APPEAR_START_PROGRESS))
        .clamp(0.0, 1.0);
    fade * fade * (3.0 - 2.0 * fade)
}

#[derive(Clone, Copy, Debug)]
struct ScopeVisualState {
    center_x: f32,
    center_y: f32,
    aperture_radius: f32,
    shadow_width: f32,
    peripheral_opacity: f32,
}
impl ScopeVisualState {
    fn from_progress(progress: f32, sight_center: Vec2) -> Self {
        let progress = progress.clamp(0.0, 1.0);
        let eased = progress * progress * (3.0 - 2.0 * progress);
        let darken_progress = ((progress - PERIPHERAL_DARKEN_START)
            / (1.0 - PERIPHERAL_DARKEN_START))
            .clamp(0.0, 1.0);
        Self {
            center_x: sight_center.x,
            center_y: sight_center.y,
            aperture_radius: SCOPE_MAX_RADIUS * eased,
            shadow_width: SCOPE_MAX_SHADOW_WIDTH * progress * (1.0 - progress),
            peripheral_opacity: PERIPHERAL_MAX_OPACITY
                * darken_progress.powf(PERIPHERAL_DARKEN_CURVE),
        }
    }
}

fn scope_shadow(mesh: &mut Scene, aspect: f32, state: ScopeVisualState) {
    let inner = state.aperture_radius;
    let shadow_outer = inner + state.shadow_width;
    let shadow_peak_width = SCOPE_MAX_SHADOW_WIDTH * 0.25;
    let inner_opacity =
        SCOPE_SHADOW_PEAK_ALPHA * (state.shadow_width / shadow_peak_width).clamp(0.0, 1.0);
    let mut angles: Vec<f32> = (0..SCOPE_SHADOW_SEGMENTS)
        .map(|i| i as f32 * std::f32::consts::TAU / SCOPE_SHADOW_SEGMENTS as f32)
        .collect();
    angles.extend([
        (1.0 - state.center_y)
            .atan2(aspect - state.center_x)
            .rem_euclid(std::f32::consts::TAU),
        (1.0 - state.center_y)
            .atan2(-aspect - state.center_x)
            .rem_euclid(std::f32::consts::TAU),
        (-1.0 - state.center_y)
            .atan2(-aspect - state.center_x)
            .rem_euclid(std::f32::consts::TAU),
        (-1.0 - state.center_y)
            .atan2(aspect - state.center_x)
            .rem_euclid(std::f32::consts::TAU),
    ]);
    for radius in [inner, shadow_outer] {
        add_scope_edge_intersections(&mut angles, radius, aspect, state.center_x, state.center_y);
    }
    angles.sort_by(f32::total_cmp);
    angles.dedup_by(|a, b| (*a - *b).abs() < 1e-5);

    let edge_radius = |angle: f32| {
        let (c, s) = (angle.cos(), angle.sin());
        let to_x_edge = if c > 0.0 {
            (aspect - state.center_x) / c
        } else if c < 0.0 {
            (-aspect - state.center_x) / c
        } else {
            f32::INFINITY
        };
        let to_y_edge = if s > 0.0 {
            (1.0 - state.center_y) / s
        } else if s < 0.0 {
            (-1.0 - state.center_y) / s
        } else {
            f32::INFINITY
        };
        to_x_edge.min(to_y_edge)
    };
    let at_angle = |angle: f32, radius: f32| {
        [
            (state.center_x + angle.cos() * radius) / aspect,
            state.center_y + angle.sin() * radius,
        ]
    };
    let point_at_edge = |angle: f32| at_angle(angle, edge_radius(angle));

    for i in 0..angles.len() {
        let a0 = angles[i];
        let a1 = if i + 1 < angles.len() {
            angles[i + 1]
        } else {
            angles[0] + std::f32::consts::TAU
        };
        let mid_edge = edge_radius((a0 + a1) * 0.5);
        if inner >= mid_edge {
            continue;
        }
        let ring_end_0 = shadow_outer.min(edge_radius(a0));
        let ring_end_1 = shadow_outer.min(edge_radius(a1));
        if state.shadow_width > 0.0 && ring_end_0 > inner && ring_end_1 > inner {
            push_scope_mask_quad(
                mesh,
                [
                    at_angle(a0, inner),
                    at_angle(a0, ring_end_0),
                    at_angle(a1, ring_end_1),
                    at_angle(a1, inner),
                ],
                [inner_opacity, 0.0, 0.0, inner_opacity],
            );
        }
        if state.peripheral_opacity > 0.0 && shadow_outer < mid_edge {
            let fill_start_0 = shadow_outer.min(edge_radius(a0));
            let fill_start_1 = shadow_outer.min(edge_radius(a1));
            push_scope_mask_quad(
                mesh,
                [
                    at_angle(a0, fill_start_0),
                    point_at_edge(a0),
                    point_at_edge(a1),
                    at_angle(a1, fill_start_1),
                ],
                [state.peripheral_opacity; 4],
            );
        }
    }
}

fn add_scope_edge_intersections(
    angles: &mut Vec<f32>,
    radius: f32,
    aspect: f32,
    center_x: f32,
    center_y: f32,
) {
    for edge_x in [-aspect, aspect] {
        let dx = edge_x - center_x;
        let remaining = radius * radius - dx * dx;
        if remaining >= 0.0 {
            let dy = remaining.sqrt();
            for relative_y in [-dy, dy] {
                if (-1.0..=1.0).contains(&(center_y + relative_y)) {
                    angles.push(relative_y.atan2(dx).rem_euclid(std::f32::consts::TAU));
                }
            }
        }
    }
    for edge_y in [-1.0, 1.0] {
        let dy = edge_y - center_y;
        let remaining = radius * radius - dy * dy;
        if remaining >= 0.0 {
            let dx = remaining.sqrt();
            for relative_x in [-dx, dx] {
                if (-aspect..=aspect).contains(&(center_x + relative_x)) {
                    angles.push(dy.atan2(relative_x).rem_euclid(std::f32::consts::TAU));
                }
            }
        }
    }
}

fn push_scope_mask_quad(mesh: &mut Scene, points: [[f32; 2]; 4], opacity: [f32; 4]) {
    let base = mesh.vertices.len() as u16;
    for (position, alpha) in points.into_iter().zip(opacity) {
        mesh.vertices.push(Vertex {
            position: [position[0], position[1], 0.0],
            color: [0.0, 0.0, 0.0, alpha],
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn scope_glass_reflection(mesh: &mut Scene, aspect: f32, state: ScopeVisualState, progress: f32) {
    let progress = progress.clamp(0.0, 1.0);
    let envelope = 4.0 * progress * (1.0 - progress);
    if envelope <= f32::EPSILON || state.aperture_radius <= f32::EPSILON {
        return;
    }

    let radius = state.aperture_radius;
    let center_x = state.center_x - radius * 0.14;
    let center_y = state.center_y + radius * 0.12;
    let radius_x = radius * 0.34;
    let radius_y = radius * 0.12;
    let rotation = -0.45_f32;
    let opacity = SCOPE_REFLECTION_MAX_OPACITY * envelope;
    let color = [0.78, 0.88, 0.92];
    let point = |angle: f32, scale: f32| {
        let x = angle.cos() * radius_x * scale;
        let y = angle.sin() * radius_y * scale;
        [
            (center_x + x * rotation.cos() - y * rotation.sin()) / aspect,
            center_y + x * rotation.sin() + y * rotation.cos(),
        ]
    };

    for i in 0..SCOPE_REFLECTION_SEGMENTS {
        let angle0 = i as f32 * std::f32::consts::TAU / SCOPE_REFLECTION_SEGMENTS as f32;
        let angle1 = (i + 1) as f32 * std::f32::consts::TAU / SCOPE_REFLECTION_SEGMENTS as f32;
        let inner0 = point(angle0, 0.45);
        let inner1 = point(angle1, 0.45);
        let outer0 = point(angle0, 1.0);
        let outer1 = point(angle1, 1.0);
        push_scope_reflection_triangle(
            mesh,
            [center_x / aspect, center_y],
            inner0,
            inner1,
            color,
            [opacity; 3],
        );
        push_scope_reflection_quad(
            mesh,
            [inner0, outer0, outer1, inner1],
            color,
            [opacity, 0.0, 0.0, opacity],
        );
    }
}

fn scope_glass_haze(mesh: &mut Scene, aspect: f32, state: ScopeVisualState, progress: f32) {
    let progress = progress.clamp(0.0, 1.0);
    let rise = smoothstep(progress / 0.035);
    let clear = 1.0
        - smoothstep(
            (progress - SCOPE_GLASS_CLEAR_START_PROGRESS)
                / (SCOPE_GLASS_CLEAR_END_PROGRESS - SCOPE_GLASS_CLEAR_START_PROGRESS),
        );
    let opacity = SCOPE_GLASS_HAZE_MAX_OPACITY * rise * clear;
    if opacity <= f32::EPSILON || state.aperture_radius <= f32::EPSILON {
        return;
    }

    let center = [state.center_x / aspect, state.center_y];
    let radius = state.aperture_radius.max(SCOPE_GLASS_HAZE_MIN_RADIUS);
    for i in 0..SCOPE_GLASS_HAZE_SEGMENTS {
        let angle0 = i as f32 * std::f32::consts::TAU / SCOPE_GLASS_HAZE_SEGMENTS as f32;
        let angle1 = (i + 1) as f32 * std::f32::consts::TAU / SCOPE_GLASS_HAZE_SEGMENTS as f32;
        let at_angle = |angle: f32| {
            [
                (state.center_x + angle.cos() * radius) / aspect,
                state.center_y + angle.sin() * radius,
            ]
        };
        push_scope_glass_triangle(mesh, center, at_angle(angle0), at_angle(angle1), opacity);
    }
}

fn push_scope_glass_triangle(
    mesh: &mut Scene,
    center: [f32; 2],
    point0: [f32; 2],
    point1: [f32; 2],
    opacity: f32,
) {
    let base = mesh.vertices.len() as u16;
    for (position, color) in [
        (center, SCOPE_GLASS_HAZE_CENTER_COLOR),
        (point0, SCOPE_GLASS_HAZE_EDGE_COLOR),
        (point1, SCOPE_GLASS_HAZE_EDGE_COLOR),
    ] {
        mesh.vertices.push(Vertex {
            position: [position[0], position[1], 0.0],
            color: [color[0], color[1], color[2], opacity],
        });
    }
    mesh.indices.extend([base, base + 1, base + 2]);
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn push_scope_reflection_triangle(
    mesh: &mut Scene,
    center: [f32; 2],
    point0: [f32; 2],
    point1: [f32; 2],
    color: [f32; 3],
    alpha: [f32; 3],
) {
    let base = mesh.vertices.len() as u16;
    for (position, opacity) in [center, point0, point1].into_iter().zip(alpha) {
        mesh.vertices.push(Vertex {
            position: [position[0], position[1], 0.0],
            color: [color[0], color[1], color[2], opacity],
        });
    }
    mesh.indices.extend([base, base + 1, base + 2]);
}

fn push_scope_reflection_quad(
    mesh: &mut Scene,
    points: [[f32; 2]; 4],
    color: [f32; 3],
    alpha: [f32; 4],
) {
    let base = mesh.vertices.len() as u16;
    for (position, opacity) in points.into_iter().zip(alpha) {
        mesh.vertices.push(Vertex {
            position: [position[0], position[1], 0.0],
            color: [color[0], color[1], color[2], opacity],
        });
    }
    mesh.indices
        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn scope_picture(mesh: &mut Scene, aspect: f32, progress: f32, sight_center: Vec2) {
    let state = ScopeVisualState::from_progress(progress, sight_center);
    if state.aperture_radius == 0.0 {
        return;
    }
    scope_shadow(mesh, aspect, state);
    scope_glass_haze(mesh, aspect, state, progress);
    scope_glass_reflection(mesh, aspect, state, progress);
    let radius = state.aperture_radius;
    let tick = radius * SCOPE_TICK_SIZE_RATIO;
    for i in 0..SCOPE_RING_SEGMENTS {
        let angle = i as f32 * std::f32::consts::TAU / SCOPE_RING_SEGMENTS as f32;
        rect(
            mesh,
            state.center_x + angle.cos() * radius - tick * 0.5,
            state.center_y + angle.sin() * radius - tick * 0.5,
            tick,
            tick,
            [0.12, 0.15, 0.17],
            aspect,
        );
    }
    let arm = radius.min(SCOPE_MAX_RADIUS) * SCOPE_RETICLE_ARM;
    let line = radius * SCOPE_RETICLE_LINE_RATIO;
    let reticle_opacity = scope_reticle_opacity(progress);
    if reticle_opacity <= f32::EPSILON {
        return;
    }
    rect_alpha(
        mesh,
        state.center_x - line * 0.5,
        state.center_y - arm,
        line,
        arm * 2.0,
        [0.14, 0.16, 0.18, reticle_opacity],
        aspect,
    );
    rect_alpha(
        mesh,
        state.center_x - arm,
        state.center_y - line * 0.5,
        arm * 2.0,
        line,
        [0.14, 0.16, 0.18, reticle_opacity],
        aspect,
    );
}

pub fn hud_mesh(weapon: &Weapon, combat: &Combat, aspect: f32, weapon_y_offset: f32) -> Scene {
    hud_mesh_with_menu(weapon, combat, aspect, weapon_y_offset, None)
}

pub fn hud_mesh_with_menu(
    weapon: &Weapon,
    combat: &Combat,
    aspect: f32,
    weapon_y_offset: f32,
    menu: Option<(&crate::persistence::GameSettings, usize)>,
) -> Scene {
    hud_mesh_with_menu_and_ammo(weapon, combat, aspect, weapon_y_offset, menu, false)
}

pub fn hud_mesh_with_menu_and_ammo(
    weapon: &Weapon,
    combat: &Combat,
    aspect: f32,
    weapon_y_offset: f32,
    menu: Option<(&crate::persistence::GameSettings, usize)>,
    infinite_ammo: bool,
) -> Scene {
    let mut m = Scene {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    hud_mesh_with_menu_and_ammo_into(
        &mut m,
        weapon,
        combat,
        aspect,
        weapon_y_offset,
        menu,
        infinite_ammo,
    );
    m
}

#[derive(Clone, Copy)]
pub struct HudOptions<'a> {
    pub menu: Option<(&'a crate::persistence::GameSettings, usize)>,
    pub infinite_ammo: bool,
    pub hit_marker_strength: f32,
    pub crosshair: crate::persistence::CrosshairStyle,
}

pub fn hud_mesh_with_hit_marker_strength(
    weapon: &Weapon,
    combat: &Combat,
    aspect: f32,
    weapon_y_offset: f32,
    hit_marker_strength: f32,
) -> Scene {
    let mut mesh = Scene {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    hud_mesh_with_menu_and_ammo_and_hit_marker_strength_into(
        &mut mesh,
        weapon,
        combat,
        aspect,
        weapon_y_offset,
        HudOptions {
            menu: None,
            infinite_ammo: false,
            hit_marker_strength,
            crosshair: crate::persistence::CrosshairStyle::default(),
        },
    );
    mesh
}

pub fn hud_mesh_with_menu_and_ammo_into(
    m: &mut Scene,
    weapon: &Weapon,
    combat: &Combat,
    aspect: f32,
    weapon_y_offset: f32,
    menu: Option<(&crate::persistence::GameSettings, usize)>,
    infinite_ammo: bool,
) {
    hud_mesh_with_menu_and_ammo_and_hit_marker_strength_into(
        m,
        weapon,
        combat,
        aspect,
        weapon_y_offset,
        HudOptions {
            menu,
            infinite_ammo,
            hit_marker_strength: 1.0,
            crosshair: crate::persistence::CrosshairStyle::default(),
        },
    );
}

pub fn hud_mesh_with_menu_and_ammo_and_hit_marker_strength_into(
    m: &mut Scene,
    weapon: &Weapon,
    combat: &Combat,
    aspect: f32,
    weapon_y_offset: f32,
    options: HudOptions<'_>,
) {
    let HudOptions {
        menu,
        infinite_ammo,
        hit_marker_strength,
        crosshair,
    } = options;
    m.vertices.clear();
    m.indices.clear();
    m.vertices.reserve(400);
    m.indices.reserve(600);
    // 武器はADS中も表示し続け、サイトピクチャーを別の軽量2Dメッシュで重ねる。
    {
        let layout = weapon.view_model_layout(weapon_y_offset);
        let (body_w, body_h, color) = match weapon.kind {
            WeaponKind::Pistol => (layout.body_width, layout.body_height, [0.24, 0.27, 0.30]),
            WeaponKind::AssaultRifle => (layout.body_width, layout.body_height, [0.17, 0.20, 0.22]),
            WeaponKind::Sniper => (layout.body_width, layout.body_height, [0.19, 0.22, 0.24]),
        };
        let x = layout.center_x;
        let y = layout.base_y;
        let dark = [0.10, 0.12, 0.14];
        let weapon_model_start = m.vertices.len();
        match weapon.kind {
            WeaponKind::Pistol => {
                slanted_rect(
                    m,
                    [x + 0.01, y + body_h * 0.05, 0.07, 0.055, 0.16, -0.025],
                    dark,
                    aspect,
                );
                rect(
                    m,
                    x - body_w * 0.40,
                    y + body_h * 0.48,
                    body_w * 0.80,
                    body_h * 0.45,
                    color,
                    aspect,
                );
                rect(m, x - 0.016, y + body_h * 0.90, 0.032, 0.13, dark, aspect);
            }
            WeaponKind::AssaultRifle => {
                slanted_rect(
                    m,
                    [
                        x - body_w * 0.42,
                        y + body_h * 0.30,
                        body_w * 0.42,
                        body_w * 0.34,
                        body_h * 0.28,
                        -0.02,
                    ],
                    dark,
                    aspect,
                );
                rect(m, x - 0.02, y + body_h * 0.92, 0.04, 0.22, dark, aspect);
                rect(
                    m,
                    x - body_w * 0.34,
                    y + body_h * 0.45,
                    body_w * 0.68,
                    body_h * 0.48,
                    color,
                    aspect,
                );
                slanted_rect(
                    m,
                    [
                        x - 0.01,
                        y + body_h * 0.05,
                        0.085,
                        0.065,
                        body_h * 0.42,
                        -0.02,
                    ],
                    dark,
                    aspect,
                );
                rect(m, x - 0.045, y + body_h * 0.89, 0.09, 0.045, dark, aspect);
            }
            WeaponKind::Sniper => {
                slanted_rect(
                    m,
                    [
                        x,
                        y + body_h * 0.20,
                        body_w * 0.30,
                        body_w * 0.22,
                        body_h * 0.30,
                        -0.02,
                    ],
                    dark,
                    aspect,
                );
                rect(
                    m,
                    x - 0.016,
                    y + body_h * 0.52,
                    0.032,
                    body_h * 0.66,
                    dark,
                    aspect,
                );
                rect(
                    m,
                    x - body_w * 0.34,
                    y + body_h * 0.44,
                    body_w * 0.68,
                    body_h * 0.48,
                    color,
                    aspect,
                );
                let progress = weapon.ads_progress.clamp(0.0, 1.0);
                let eased = progress * progress * (3.0 - 2.0 * progress);
                let scope_rim_opacity = 1.0 - eased;
                if scope_rim_opacity > f32::EPSILON {
                    scope_model_ring(m, layout.sight_screen, aspect, progress, scope_rim_opacity);
                }
            }
        }
        // The highlighted barrel aperture marks the same point used as the ray origin.
        triangle(
            m,
            layout.muzzle_screen.to_array(),
            [
                layout.muzzle_screen.x - 0.014,
                layout.muzzle_screen.y - 0.026,
            ],
            [
                layout.muzzle_screen.x + 0.014,
                layout.muzzle_screen.y - 0.026,
            ],
            [0.58, 0.62, 0.64],
            aspect,
        );
        if combat.feedback > 0.075 {
            rect(
                m,
                layout.muzzle_screen.x - 0.025,
                layout.muzzle_screen.y + 0.01,
                0.05,
                0.045,
                [1., 0.8, 0.2],
                aspect,
            );
        }
        if weapon.kind == WeaponKind::Sniper {
            let progress = weapon.ads_progress.clamp(0.0, 1.0);
            let eased = progress * progress * (3.0 - 2.0 * progress);
            let visibility = 1.0 - eased;
            for vertex in &mut m.vertices[weapon_model_start..] {
                vertex.color[3] *= visibility;
            }
        }
    }
    if weapon.kind == WeaponKind::Sniper {
        scope_picture(
            m,
            aspect,
            weapon.ads_progress,
            weapon.view_model_layout(weapon_y_offset).sight_screen,
        );
    }
    if weapon.ads_progress <= f32::EPSILON {
        let half = crosshair.thickness * 0.5;
        for sign in [-1.0_f32, 1.0] {
            let x = if sign > 0.0 {
                crosshair.gap
            } else {
                -crosshair.gap - crosshair.length
            };
            rect(
                m,
                x,
                -half,
                crosshair.length,
                crosshair.thickness,
                crosshair.color,
                aspect,
            );
            let y = if sign > 0.0 {
                crosshair.gap
            } else {
                -crosshair.gap - crosshair.length
            };
            rect(
                m,
                -half,
                y,
                crosshair.thickness,
                crosshair.length,
                crosshair.color,
                aspect,
            );
        }
        if crosshair.dot {
            let dot = crosshair.thickness * 1.4;
            rect(m, -dot * 0.5, -dot * 0.5, dot, dot, crosshair.color, aspect);
        }
    }
    let hit_marker_strength = if hit_marker_strength.is_finite() {
        hit_marker_strength.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if combat.feedback > 0.0 && combat.last_shot.hit && hit_marker_strength > 0.0 {
        let color = if combat.last_shot.killed {
            [1.0, 0.05, 0.06]
        } else if combat.last_shot.headshot {
            [0.98, 0.21, 0.05]
        } else {
            [1.0, 0.3, 0.24]
        };
        let (inner, outer, thickness) = if combat.last_shot.killed {
            (0.014, 0.046, 0.007)
        } else {
            (0.02, 0.033, 0.0035)
        };
        let inner = inner * hit_marker_strength;
        let outer = outer * hit_marker_strength;
        let thickness = thickness * hit_marker_strength;
        for (x, y) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
            stroke_segment(
                m,
                [x * inner, y * inner],
                [x * outer, y * outer],
                thickness,
                color,
                aspect,
            );
        }
    }
    if weapon.bolt_remaining > 0.0 {
        rect(
            m,
            -0.15,
            -0.82,
            0.3 * (1. - weapon.bolt_remaining / weapon.data.bolt_seconds),
            0.012,
            [1., 0.65, 0.25],
            aspect,
        );
    }
    if !infinite_ammo {
        digit(m, weapon.ammo / 10, aspect - 0.19, -0.87, aspect);
        digit(m, weapon.ammo % 10, aspect - 0.13, -0.87, aspect);
    }
    if weapon.reload_remaining > 0.0 {
        rect(
            m,
            -0.15,
            -0.82,
            0.3 * (1. - weapon.reload_remaining / weapon.data.reload_seconds),
            0.015,
            [0.3, 0.8, 1.],
            aspect,
        );
    }
    if let Some((settings, selected)) = menu {
        settings_overlay(m, settings, selected, aspect);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        append_notice_text, append_vertically_centered_text_line, capture_prompt_overlay,
        capture_prompt_text, credits_overlay, credits_text, result_overlay, result_text,
        scope_shadow, settings_overlay, settings_text_with_bindings, settings_visible_range,
        ResultInfo, ScopeVisualState, SCOPE_MAX_RADIUS, SETTINGS_VISIBLE_ROWS,
    };
    use crate::engine::text::TextMesh;
    use crate::simulation::game_mode::SessionMode;
    use crate::simulation::weapon::{Weapon, WeaponKind, SNIPER};

    #[test]
    fn settings_scroll_window_keeps_selection_visible_at_each_end() {
        assert_eq!(SETTINGS_VISIBLE_ROWS, 11);
        assert_eq!(settings_visible_range(0, 24), 0..11);
        assert_eq!(settings_visible_range(10, 24), 5..16);
        assert_eq!(settings_visible_range(23, 24), 13..24);
    }

    #[test]
    fn change_mode_row_remains_visible_and_highlighted() {
        let settings = crate::persistence::GameSettings::default();
        let change_mode =
            settings_text_with_bindings(&settings, 23, [1280, 720], false, 0, false, 0);
        assert!(change_mode
            .vertices
            .iter()
            .any(|vertex| vertex.color == [1.0, 0.65, 0.24, 1.0]));
    }

    #[test]
    fn selected_background_covers_the_rendered_text_with_balanced_padding() {
        let settings = crate::persistence::GameSettings::default();
        let text = settings_text_with_bindings(&settings, 1, [1280, 720], false, 0, false, 0);
        let selected_text = text
            .vertices
            .iter()
            .filter(|vertex| vertex.color == [1.0, 0.65, 0.24, 1.0])
            .collect::<Vec<_>>();
        let glyph_min_y = selected_text
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        let glyph_max_y = selected_text
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let glyph_min_x = selected_text
            .iter()
            .map(|vertex| vertex.position[0])
            .fold(f32::INFINITY, f32::min);
        let glyph_max_x = selected_text
            .iter()
            .map(|vertex| vertex.position[0])
            .fold(f32::NEG_INFINITY, f32::max);

        let mut background = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        settings_overlay(&mut background, &settings, 1, 16.0 / 9.0);
        let highlight = background
            .vertices
            .iter()
            .filter(|vertex| vertex.color == [0.92, 0.48, 0.16, 0.22])
            .collect::<Vec<_>>();
        let background_min_y = highlight
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        let background_max_y = highlight
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let aspect = 16.0 / 9.0;
        let background_min_x = highlight
            .iter()
            .map(|vertex| vertex.position[0] * aspect)
            .fold(f32::INFINITY, f32::min);
        let background_max_x = highlight
            .iter()
            .map(|vertex| vertex.position[0] * aspect)
            .fold(f32::NEG_INFINITY, f32::max);

        assert!(background_min_y < glyph_min_y - 0.005);
        assert!(background_max_y > glyph_max_y + 0.005);
        assert!(background_min_x < glyph_min_x - 0.005);
        assert!(background_max_x > glyph_max_x + 0.005);
        let top_padding = background_max_y - glyph_max_y;
        let bottom_padding = glyph_min_y - background_min_y;
        assert!((top_padding - bottom_padding).abs() < 0.02);
    }

    #[test]
    fn sensitivity_slider_stays_clear_of_its_value_text() {
        let settings = crate::persistence::GameSettings::default();
        let aspect = 16.0 / 9.0;
        let text = settings_text_with_bindings(&settings, 1, [1280, 720], false, 0, false, 0);
        let value_start_x = text
            .vertices
            .iter()
            .filter(|vertex| vertex.color == [1.0, 0.65, 0.24, 1.0] && vertex.position[0] > 0.0)
            .map(|vertex| vertex.position[0])
            .fold(f32::INFINITY, f32::min);

        let mut background = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        settings_overlay(&mut background, &settings, 1, aspect);
        let slider_end_x = background
            .vertices
            .iter()
            .filter(|vertex| vertex.color == [1.0, 0.65, 0.24, 1.0])
            .map(|vertex| vertex.position[0] * aspect)
            .fold(f32::NEG_INFINITY, f32::max);

        assert!(slider_end_x < value_start_x - 0.02);
    }

    #[test]
    fn storage_notice_adds_readable_text_to_the_hud_mesh() {
        let mut mesh = TextMesh::default();
        append_notice_text(
            &mut mesh,
            "SETTINGS NOT SAVED - CHECK STORAGE",
            [1280, 720],
            0.575,
        );

        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.indices.len() % 6, 0);
        assert!(mesh.vertices.iter().all(|vertex| vertex.color[0] == 1.0));
        let center_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .sum::<f32>()
            / mesh.vertices.len() as f32;
        assert!((center_y - 0.575).abs() < 0.05);
    }

    fn sniper_scope_state(progress: f32) -> ScopeVisualState {
        let mut weapon = Weapon::new(WeaponKind::Sniper, SNIPER);
        weapon.ads_progress = progress.clamp(0.0, 1.0);
        let sight_center = weapon.view_model_layout(0.0).sight_screen;
        ScopeVisualState::from_progress(progress, sight_center)
    }

    fn mask_covers_point(mesh: &crate::Scene, point: [f32; 2]) -> bool {
        mesh.indices.as_chunks::<3>().0.iter().any(|triangle| {
            let vertices = triangle.map(|index| mesh.vertices[index as usize]);
            let sides = [
                (point[0] - vertices[0].position[0])
                    * (vertices[1].position[1] - vertices[0].position[1])
                    - (point[1] - vertices[0].position[1])
                        * (vertices[1].position[0] - vertices[0].position[0]),
                (point[0] - vertices[1].position[0])
                    * (vertices[2].position[1] - vertices[1].position[1])
                    - (point[1] - vertices[1].position[1])
                        * (vertices[2].position[0] - vertices[1].position[0]),
                (point[0] - vertices[2].position[0])
                    * (vertices[0].position[1] - vertices[2].position[1])
                    - (point[1] - vertices[2].position[1])
                        * (vertices[0].position[0] - vertices[2].position[0]),
            ];
            sides.iter().all(|side| *side >= 0.0) || sides.iter().all(|side| *side <= 0.0)
        })
    }

    #[test]
    fn scope_view_grows_smoothly_while_eye_alignment_shadow_fades() {
        let hip = sniper_scope_state(0.0);
        let early = sniper_scope_state(0.3);
        let middle = sniper_scope_state(0.5);
        let late = sniper_scope_state(0.7);
        let aligned = sniper_scope_state(1.0);

        assert_eq!(hip.aperture_radius, 0.0);
        assert!(early.aperture_radius < middle.aperture_radius);
        assert!(middle.aperture_radius < late.aperture_radius);
        assert!(late.aperture_radius < aligned.aperture_radius);
        assert!(early.shadow_width > 0.0);
        assert!(middle.shadow_width > early.shadow_width);
        assert!(late.shadow_width < middle.shadow_width);
        assert_eq!(aligned.shadow_width, 0.0);
        assert!(hip.center_x > early.center_x);
        assert!(early.center_x > middle.center_x);
        assert!(middle.center_x > aligned.center_x);
        assert!(hip.center_y < early.center_y);
        assert!(early.center_y < middle.center_y);
        assert!(middle.center_y < aligned.center_y);
        assert!(aligned.center_y.abs() < f32::EPSILON);
    }

    #[test]
    fn fully_aimed_scope_view_fills_the_display_instead_of_looking_like_a_small_lens() {
        let aligned = sniper_scope_state(1.0);
        assert!(aligned.aperture_radius > 1.0);
        assert!(aligned.aperture_radius > 16.0 / 9.0);
        assert_eq!(aligned.aperture_radius, SCOPE_MAX_RADIUS);
    }

    #[test]
    fn scope_geometry_is_bounded_for_out_of_range_progress() {
        let below = sniper_scope_state(-2.0);
        let above = sniper_scope_state(2.0);
        assert_eq!(below.aperture_radius, 0.0);
        assert_eq!(
            above.aperture_radius,
            sniper_scope_state(1.0).aperture_radius
        );
    }

    #[test]
    fn peripheral_opacity_begins_gradually_and_rises_to_near_black() {
        let levels = [0.0, 0.3, 0.5, 0.7, 0.9, 1.0]
            .map(|progress| sniper_scope_state(progress).peripheral_opacity);
        assert_eq!(levels[0], 0.0);
        assert_eq!(levels[1], 0.0);
        assert!(levels[2] > levels[1]);
        assert!(levels[3] > levels[2]);
        assert!(levels[4] > levels[3]);
        assert!(levels[5] > 0.95);
        assert!(levels.windows(2).all(|pair| pair[1] >= pair[0]));
    }

    #[test]
    fn peripheral_mask_keeps_scope_opening_clear_and_darkens_screen_edges() {
        let mut aligned = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        scope_shadow(&mut aligned, 16.0 / 9.0, sniper_scope_state(1.0));
        assert_eq!(aligned.vertices.len() % 4, 0);
        assert!(aligned.vertices.iter().any(|vertex| vertex.color[3] > 0.95));
        assert!(!mask_covers_point(&aligned, [0.0, 0.9]));
        assert!(mask_covers_point(&aligned, [0.99, 0.99]));

        let mut early = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        scope_shadow(&mut early, 16.0 / 9.0, sniper_scope_state(0.3));
        assert_eq!(early.vertices.len() % 4, 0);

        let mut partial = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        scope_shadow(&mut partial, 16.0 / 9.0, sniper_scope_state(0.5));
        assert_eq!(partial.vertices.len() % 4, 0);
        assert!(partial.vertices[0].color[3] > partial.vertices[1].color[3]);
        assert!(partial.vertices[4].color[3] > 0.0);
        assert_eq!(partial.vertices[4].color[3], partial.vertices[5].color[3]);
    }

    #[test]
    fn fully_scoped_peripheral_mask_reaches_all_four_screen_corners() {
        let aspect = 16.0 / 9.0;
        for progress in [0.5, 1.0] {
            let mut mesh = crate::Scene {
                vertices: Vec::new(),
                indices: Vec::new(),
            };
            scope_shadow(&mut mesh, aspect, sniper_scope_state(progress));
            for corner in [[-1.0, -1.0], [-1.0, 1.0], [1.0, -1.0], [1.0, 1.0]] {
                assert!(
                    mesh.vertices.iter().any(|vertex| {
                        (vertex.position[0] - corner[0]).abs() < 1e-5
                            && (vertex.position[1] - corner[1]).abs() < 1e-5
                    }),
                    "peripheral mask at {progress} ADS must reach corner {corner:?}"
                );
            }
        }
    }

    #[test]
    fn settings_overlay_generates_bounded_finite_hud_geometry() {
        let combat = crate::simulation::combat::Combat::default();
        let loadout = crate::simulation::weapon::Loadout::default();
        let mesh = super::hud_mesh_with_menu(
            loadout.active(),
            &combat,
            16.0 / 9.0,
            0.0,
            Some((&crate::persistence::GameSettings::default(), 0)),
        );
        assert!(!mesh.indices.is_empty());
        assert!(mesh.vertices.len() < 4096);
        assert!(mesh.indices.len() < 8192);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.position.iter().all(|v| v.is_finite())));
    }

    #[test]
    fn pause_overlay_dims_the_game_and_highlights_a_wide_menu_row() {
        let mut mesh = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        settings_overlay(
            &mut mesh,
            &crate::persistence::GameSettings::default(),
            3,
            16.0 / 9.0,
        );

        assert!(mesh.vertices[0].color[3] < 1.0);
        assert!(mesh.vertices[..4]
            .iter()
            .any(|vertex| vertex.position[0] <= -0.99));
        assert!(mesh.vertices.iter().any(|vertex| {
            vertex.color[0] > 0.9 && vertex.color[1] > 0.5 && vertex.color[2] < 0.3
        }));
        let widest_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[0].abs())
            .fold(0.0_f32, f32::max);
        assert!(widest_x >= 0.6);
    }

    #[test]
    fn sensitivity_selection_background_covers_its_label_and_slider() {
        let mut mesh = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        settings_overlay(
            &mut mesh,
            &crate::persistence::GameSettings::default(),
            1,
            16.0 / 9.0,
        );

        let highlighted = mesh
            .vertices
            .iter()
            .filter(|vertex| vertex.color == [0.92, 0.48, 0.16, 0.22])
            .collect::<Vec<_>>();
        let min_y = highlighted
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        let max_y = highlighted
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            min_y < 0.365,
            "selected row must extend below slider: {min_y}"
        );
        assert!(
            max_y > 0.405,
            "selected row must extend above label: {max_y}"
        );
    }

    #[test]
    fn resume_and_change_mode_highlights_cover_the_full_text_height() {
        for (selected, expected_min_y, expected_max_y, color) in [
            (0, 0.44, 0.50, [0.92, 0.48, 0.16, 0.22]),
            (23, -0.51, -0.45, [0.92, 0.48, 0.16, 0.22]),
        ] {
            let mut mesh = crate::Scene {
                vertices: Vec::new(),
                indices: Vec::new(),
            };
            settings_overlay(
                &mut mesh,
                &crate::persistence::GameSettings::default(),
                selected,
                16.0 / 9.0,
            );
            let selected_row = mesh
                .vertices
                .iter()
                .filter(|vertex| vertex.color == color)
                .collect::<Vec<_>>();
            let min_y = selected_row
                .iter()
                .map(|vertex| vertex.position[1])
                .fold(f32::INFINITY, f32::min);
            let max_y = selected_row
                .iter()
                .map(|vertex| vertex.position[1])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                min_y < expected_min_y,
                "row {selected} ends too high: {min_y}"
            );
            assert!(
                max_y > expected_max_y,
                "row {selected} starts too low: {max_y}"
            );
        }
    }

    #[test]
    fn title_menu_highlights_cover_each_larger_menu_label() {
        for (selected, text_top, text_height) in
            [(0, 0.18, 0.047), (2, -0.26, 0.047), (3, -0.48, 0.047)]
        {
            let mut mesh = crate::Scene {
                vertices: Vec::new(),
                indices: Vec::new(),
            };
            super::title_overlay(&mut mesh, selected, 16.0 / 9.0);
            let row = mesh
                .vertices
                .iter()
                .filter(|vertex| vertex.color == [0.92, 0.48, 0.16, 0.22])
                .collect::<Vec<_>>();
            let bottom = row
                .iter()
                .map(|vertex| vertex.position[1])
                .fold(f32::INFINITY, f32::min);
            let top = row
                .iter()
                .map(|vertex| vertex.position[1])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(bottom < text_top - text_height * 2.0);
            assert!(top > text_top);
        }
    }

    #[test]
    fn capture_prompt_draws_a_panel_and_readable_text() {
        let mut mesh = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        capture_prompt_overlay(&mut mesh, 16.0 / 9.0);
        assert!(!mesh.indices.is_empty());
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == [0.030, 0.040, 0.055, 0.96]));

        let mut text = TextMesh::default();
        capture_prompt_text(
            &mut text,
            "CLICK TO PLAY",
            "MOUSE LOOK USES POINTER LOCK",
            [1280, 720],
        );
        assert!(!text.vertices.is_empty());
    }

    #[test]
    fn credits_screen_draws_a_panel_and_text() {
        let mut mesh = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        credits_overlay(&mut mesh, 16.0 / 9.0);
        assert!(!mesh.indices.is_empty());
        let mut text = TextMesh::default();
        credits_text(&mut text, [1280, 720]);
        assert!(!text.vertices.is_empty());
    }

    #[test]
    fn result_screen_draws_a_panel_and_summary_text() {
        let mut mesh = crate::Scene {
            vertices: Vec::new(),
            indices: Vec::new(),
        };
        result_overlay(&mut mesh, 16.0 / 9.0);
        assert!(!mesh.indices.is_empty());

        let info = ResultInfo {
            mode: SessionMode::FreeTraining,
            kills: 3,
            accuracy: 0.5,
            headshot_rate: 0.25,
            seconds: 61.0,
            new_best: true,
        };
        let mut text = TextMesh::default();
        result_text(&mut text, &info, [1280, 720]);
        let first = text.vertices.len();
        assert!(first > 0);
        // Reusing the buffer must replace, not append to, the previous text.
        result_text(&mut text, &info, [1280, 720]);
        assert_eq!(text.vertices.len(), first);
    }

    #[test]
    fn vertically_centered_line_puts_glyph_center_on_target() {
        let font = crate::engine::text::FontAtlas::shared();
        for (text, target) in [("FREE TRAINING", 0.18_f32), ("SETTINGS", -0.26)] {
            let mut mesh = TextMesh::default();
            append_vertically_centered_text_line(
                &mut mesh,
                font,
                text,
                target,
                0.047,
                [1280, 720],
                [1.0; 4],
            );
            let (min_y, max_y) = mesh.vertices.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY),
                |(min_y, max_y), vertex| {
                    (min_y.min(vertex.position[1]), max_y.max(vertex.position[1]))
                },
            );
            assert!(
                (min_y + max_y) * 0.5 - target < 1e-4,
                "{text} center should match {target}"
            );
            assert!((min_y + max_y) * 0.5 - target > -1e-4);
        }
    }
}
