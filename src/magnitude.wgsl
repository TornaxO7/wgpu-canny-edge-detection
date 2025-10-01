@group(0) @binding(0)
var vertical: texture_2d<f32>;

@group(0) @binding(1)
var horizontal: texture_2d<f32>;

@group(0) @binding(2)
var magnitude: texture_storage_2d<r32float, write>;

@group(0) @binding(3)
var radian: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let v = textureLoad(vertical, gid.xy, 0).r;
    let h = textureLoad(horizontal, gid.xy, 0).r;

    let mag = sqrt(v*v + h*h);
    textureStore(magnitude, gid.xy, vec4f(mag, 0., 0., 1.));

    var rad = 0.;
    // avoid suprises
    if (v != 0.) {
        rad = atan2(v, h);
    }

    textureStore(radian, gid.xy, vec4f(rad, 0., 0., 1.));
}
