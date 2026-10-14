struct Uniforms { vp: mat4x4<f32> }
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world: vec3<f32>,
}
@vertex fn vs(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> Output {
    var out: Output;
    out.position = uniforms.vp * vec4<f32>(position, 1.0);
    out.color = color;
    out.world = position;
    return out;
}
// 壁面識別用の細い格子線。面の法線をワールド座標の微分から求め、
// 水平面はワールドx/z、垂直面は高さyを含む格子を描く。
// 繰り返し1m、線幅3cm、明暗差は控えめ(0.94)。
@fragment fn fs(in: Output) -> @location(0) vec4<f32> {
    let wx = dpdx(in.world);
    let wy = dpdy(in.world);
    let normal = normalize(cross(wx, wy));
    let is_horizontal = abs(normal.y) > 0.5;
    let grid = 1.0;
    let line_width = 0.03;
    var d: f32;
    if is_horizontal {
        // 水平面（床・上面・下面）: x/zの格子。
        let cell_x = fract(in.world.x / grid);
        let cell_z = fract(in.world.z / grid);
        let near_x = min(cell_x, 1.0 - cell_x);
        let near_z = min(cell_z, 1.0 - cell_z);
        d = min(near_x, near_z);
    } else {
        // 垂直面（壁・遮蔽物の側面）: 高さyの水平線とx/zの縦線。
        let cell_x = fract(in.world.x / grid);
        let cell_y = fract(in.world.y / grid);
        let cell_z = fract(in.world.z / grid);
        let near_x = min(cell_x, 1.0 - cell_x);
        let near_y = min(cell_y, 1.0 - cell_y);
        let near_z = min(cell_z, 1.0 - cell_z);
        d = min(min(near_x, near_y), near_z);
    }
    var shade = 1.0;
    if d < line_width {
        shade = 0.94;
    }
    return vec4<f32>(in.color * shade, 1.0);
}