@group(0) @binding(0)
var input: texture_2d<f32>;

@group(0) @binding(1)
var output: texture_storage_2d<r32float, write>;

@group(0) @binding(2)
var<storage, read_write> max_value: u32;

struct Thresholds {
    top: f32,
    bottom: f32,
};

@group(0) @binding(3)
var<uniform> thresholds: Thresholds;

const u32_MAX: f32 = 4294967295.;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let presence = textureLoad(input, gid.xy, 0).r;

    let upper = (f32(max_value) * thresholds.top) / u32_MAX;
    let lower = upper * thresholds.bottom;

    var value = 0.;
    if (presence >= upper) {
        value = 1.;
    } else if (presence < lower) {
        value = 0.;
    } else {
        value = 0.5;
    }

    textureStore(output, gid.xy, vec4f(value, 0., 0., 1.));
}
