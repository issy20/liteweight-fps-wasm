struct Input {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@group(0) @binding(0) var font_atlas: texture_2d<f32>;
@group(0) @binding(1) var font_sampler: sampler;

@vertex
fn vs(input: Input) -> Output {
    var output: Output;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@fragment
fn fs(input: Output) -> @location(0) vec4<f32> {
    let coverage = textureSample(font_atlas, font_sampler, input.uv).r;
    return vec4<f32>(input.color.rgb, input.color.a * coverage);
}
