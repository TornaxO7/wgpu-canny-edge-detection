@group(0) @binding(0)
var input: texture_2d<f32>;

@group(0) @binding(1)
var output: texture_storage_2d<r32float, write>;

@group(0) @binding(2)
var<uniform> kernel: array<vec4f, 3>;

@compute
@workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let igid = vec2i(gid.xy);

    let input_size: vec2i = vec2i(textureDimensions(input));

    var sum: f32 = 0.;
    for (var x = -1; x < 2; x++) {
        for (var y = -1; y < 2; y++) {
            let coords: vec2i = igid + vec2i(x, y);

            let is_over_the_top = coords.y < 0;
            let is_over_left_edge = coords.x < 0;
            let is_over_right_edge = coords.x >= input_size.x;
            let is_below_bottom_edge = coords.y >= input_size.y;
            if (is_over_the_top || is_over_left_edge || is_over_right_edge || is_below_bottom_edge) {
                return;
            }

            sum += kernel[x + 1][y + 1] * textureLoad(input, coords, 0).r;
        }
    }

    sum = clamp(0., 1., sum);
    textureStore(output, igid, vec4f(sum, 0., 0., 1.));
}
