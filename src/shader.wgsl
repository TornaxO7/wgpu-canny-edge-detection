@group(0) @binding(0)
var input: texture_2d<f32>;

@group(0) @binding(1)
var output: texture_storage_2d<rgba8unorm, write>;

struct InputSize {
    width: i32,
    height: i32,
};

@group(0) @binding(2)
var<uniform> input_size: InputSize;

@compute
@workgroup_size(16, 16, 1)
fn gaussian_filter(@builtin(global_invocation_id) gid: vec3u) {
    const kernel = array(
        array(.075, .124, .075),
        array(.124, .204, .124),
        array(.075, .124, .075),
    );

    apply_kernel(gid, kernel);
}

@compute
@workgroup_size(16, 16, 1)
fn soeber_vertical(@builtin(global_invocation_id) gid: vec3u) {
    const kernel = array(
        array(-1., -2., -1.),
        array( 0.,  0.,  0.),
        array( 1.,  2.,  1.),
    );

    apply_kernel(gid, kernel);
}

@compute
@workgroup_size(16, 16, 1)
fn soeber_horizontal(@builtin(global_invocation_id) gid: vec3u) {
    const kernel = array(
        array(-1., 0., 1.),
        array(-2., 0., 2.),
        array(-1., 0., 1.),
    );

    apply_kernel(gid, kernel);
}

fn apply_kernel(gid: vec3u, kernel: array<array<f32, 3>, 3>) {
    let igid = vec2i(gid.xy);

    var sum: vec3f = vec3f(0.);
    for (var x = -1; x < 2; x++) {
        for (var y = -1; y < 2; y++) {
            let coords: vec2i = igid + vec2i(x, y);

            let is_over_the_top = coords.y < 0;
            let is_over_left_edge = coords.x < 0;
            let is_over_right_edge = coords.x >= input_size.width;
            let is_below_bottom_edge = coords.y >= input_size.height;
            if (is_over_the_top || is_over_left_edge || is_over_right_edge || is_below_bottom_edge) {
                return;
            }

            sum += kernel[x + 1][y + 1] * textureLoad(input, coords, 0).rgb;
        }
    }

    sum = clamp(vec3f(0.), vec3f(1.), sum);

    let input_pixel = textureLoad(input, igid, 0);
    textureStore(output, igid, vec4f(sum, input_pixel.a));
}
