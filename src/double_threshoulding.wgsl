@group(0) @binding(0)
var input: texture_2d<f32>;

@group(0) @binding(1)
var output: texture_storage_2d<r32float, write>;

struct Thresholds {
    top: f32,
    bottom: f32,
};

@roup(0) @binding(2)
var<uniform> thresholds: Thresholds;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    
}
