@group(0) @binding(0)
var input: texture_2d<f32>;

@group(0) @binding(1)
var output: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let pixel = textureLoad(input, gid.xy, 0);

    let luminance = 0.3 * pixel.r + 0.59 * pixel.g + 0.11 * pixel.b;
    textureStore(output, gid.xy, vec4f(luminance, 0., 0., 1.));
}

