@group(0) @binding(0)
var magnitudes: texture_2d<f32>;

@group(0) @binding(1)
var radians: texture_2d<f32>;

@group(0) @binding(2)
var output: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let magnitude = textureLoad(magnitudes, gid.xy, 0).r;
    let radian = textureLoad(radians, gid.xy, 0).r;

    let coord = vec2f(gid.xy);
    let dir = vec2f(cos(radian), sin(radian));

    let p1 = coord + dir;
    let p2 = coord - dir;

    let m1 = bilinear_interpolation(p1);
    let m2 = bilinear_interpolation(p2);

    if (m1 < magnitude && m2 < magnitude) {
        textureStore(output, gid.xy, vec4f(magnitude, 0., 0., 1.));
    }
}

fn bilinear_interpolation(p: vec2f) -> f32 {
    let id = vec2u(floor(p));

    // get the theoretical positions
    let tl = id + vec2u(0, 0);
    let tr = id + vec2u(1, 0);
    let bl = id + vec2u(0, 1);
    let br = id + vec2u(1, 1);

    // skip, if p is at the edge of the whole texture
    if (is_in_texture(tl) && is_in_texture(tr) && is_in_texture(bl) && is_in_texture(br)) {
        let sgv = smoothstep(vec2f(0.), vec2f(1.), fract(p));

        let tlm = textureLoad(magnitudes, tl, 0).r;
        let trm = textureLoad(magnitudes, tr, 0).r;
        let blm = textureLoad(magnitudes, bl, 0).r;
        let brm = textureLoad(magnitudes, br, 0).r;

        let m1 = mix(tlm, trm, sgv.x);
        let m2 = mix(blm, brm, sgv.x);
        return mix(m1, m2, sgv.y);
    } else {
        // basically early exit
        return 1e10;
    }
}

fn is_in_texture(pixel_coord: vec2u) -> bool {
    let p = vec2f(pixel_coord);
    let sizeu: vec2u = textureDimensions(output);
    let sizef: vec2f = vec2f(sizeu);

    let x_is_valid = p.x >= 0. && p.x < sizef.x;
    let y_is_valid = p.y >= 0. && p.y < sizef.y;

    return x_is_valid && y_is_valid;
}
