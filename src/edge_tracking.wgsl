@group(0) @binding(0)
var img: texture_storage_2d<r32float, read_write>;

const IS_EDGE: f32 = 1.0;
const NOT_EDGE: f32 = 0.0;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let curr_value = textureLoad(img, gid.xy).r;
    if (curr_value == IS_EDGE) {
        textureStore(img, gid.xy, vec4f(IS_EDGE, 0., 0., 1.));
        return;
    } else if (curr_value == NOT_EDGE) {
        return;
    }

    for (var x = -1; x < 2; x++) {
        for (var y = -1; y < 2; y++) {
            let coord = vec2i(gid.xy) + vec2i(x, y);

            if is_in_texture(coord) {
                let value = textureLoad(img, coord).r;

                if (value == IS_EDGE) {
                    textureStore(img, gid.xy, vec4f(IS_EDGE, 0., 0., 1.));
                    return;
                }
            }
        }
    }
}

fn is_in_texture(coord: vec2i) -> bool {
    let size: vec2i = vec2i(textureDimensions(img));

    let x_is_valid = coord.x >= 0 && coord.x < size.x;
    let y_is_valid = coord.y >= 0 && coord.y < size.y;

    return x_is_valid && y_is_valid;
}
