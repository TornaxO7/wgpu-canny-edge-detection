@group(0) @binding(0)
var input: texture_2d<f32>;

@group(0) @binding(1)
var<storage, read_write> max_value: atomic<u32>;

const u32_MAX: f32 = 4294967295.;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let value = textureLoad(input, gid.xy, 0).r;

    atomicMax(&max_value, u32(floor(value * u32_MAX)));
}
