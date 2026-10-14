use glam::Vec3;

/// 衝突判定用の軸平行境界ボックス。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}
impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
    /// 高さ位置と半径を与えられた円筒（水平円+垂直線分）がこのAABBと交差するかを返す。
    pub fn hits_cylinder(&self, center: Vec3, radius: f32, height: f32) -> bool {
        let half = height * 0.5;
        let cy_min = center.y - half;
        let cy_max = center.y + half;
        if cy_max < self.min.y || cy_min > self.max.y {
            return false;
        }
        let cx = center.x.clamp(self.min.x, self.max.x);
        let cz = center.z.clamp(self.min.z, self.max.z);
        (center.x - cx) * (center.x - cx) + (center.z - cz) * (center.z - cz) < radius * radius
    }
}

impl Aabb {
    /// 正規化した射線の始点から最初の交点までの距離。内部始点は0。
    pub fn ray_distance(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        if !origin.is_finite() || !direction.is_finite() || direction.length_squared() < 1e-12 {
            return None;
        }
        let direction = direction.normalize();
        let mut near = 0.0_f32;
        let mut far = f32::INFINITY;
        for axis in 0..3 {
            if direction[axis].abs() < 1e-8 {
                if origin[axis] < self.min[axis] || origin[axis] > self.max[axis] {
                    return None;
                }
            } else {
                let a = (self.min[axis] - origin[axis]) / direction[axis];
                let b = (self.max[axis] - origin[axis]) / direction[axis];
                near = near.max(a.min(b));
                far = far.min(a.max(b));
                if near > far {
                    return None;
                }
            }
        }
        Some(near)
    }
}
