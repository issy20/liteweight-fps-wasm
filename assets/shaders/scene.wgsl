struct Uniforms { vp: mat4x4<f32> }
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}
@vertex fn vs(@location(0) position: vec3<f32>, @location(1) color: vec4<f32>) -> Output {
    var out: Output;
    out.position = uniforms.vp * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}
@fragment fn fs(in: Output) -> @location(0) vec4<f32> {
    return in.color;
}
