@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var nearest: sampler;
struct Output { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs(@builtin(vertex_index) i: u32) -> Output {
    let positions = array<vec2<f32>, 3>(vec2<f32>(-1., -1.), vec2<f32>(3., -1.), vec2<f32>(-1., 3.));
    var out: Output;
    out.position = vec4<f32>(positions[i], 0., 1.);
    out.uv = vec2<f32>(positions[i].x * 0.5 + 0.5, 0.5 - positions[i].y * 0.5);
    return out;
}
@fragment fn fs(in: Output) -> @location(0) vec4<f32> { return textureSample(scene, nearest, in.uv); }
