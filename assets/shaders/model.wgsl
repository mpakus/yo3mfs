struct Camera { view_projection: mat4x4<f32>, options: vec4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) color: vec3<f32>,
};

fn linear_rgb(srgb: vec3<f32>) -> vec3<f32> {
    return select(pow((srgb + 0.055) / 1.055, vec3<f32>(2.4)),
                  srgb / 12.92, srgb <= vec3<f32>(0.04045));
}

@vertex
fn vs_main(@location(0) position: vec3<f32>,
           @location(1) col0: vec4<f32>, @location(2) col1: vec4<f32>,
           @location(3) col2: vec4<f32>, @location(4) col3: vec4<f32>,
           @location(5) base_color: u32, @location(6) paint_color: u32) -> VertexOutput {
    let world = mat4x4<f32>(col0, col1, col2, col3) * vec4<f32>(position, 1.0);
    var out: VertexOutput;
    out.clip = camera.view_projection * world;
    out.world = world.xyz;
    let packed = select(base_color, paint_color, paint_color != 0u);
    let srgb = vec3<f32>(f32(packed & 255u), f32((packed >> 8u) & 255u),
                         f32((packed >> 16u) & 255u)) / 255.0;
    // Interpolate vertex colors in linear light; uniform Bambu facet colors stay uniform.
    out.color = linear_rgb(srgb);
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // World derivatives preserve flat faces under reflection and nonuniform scaling.
    let normal = normalize(cross(dpdx(input.world), dpdy(input.world)));
    let key = abs(dot(normal, normalize(vec3<f32>(-0.4, -0.7, 1.0))));
    let fill = abs(dot(normal, normalize(vec3<f32>(0.8, 0.3, 0.2))));
    let light = 0.32 + 0.5 * key + 0.18 * fill;
    var linear = linear_rgb(vec3<f32>(174.0, 194.0, 209.0) / 255.0);
    if camera.options.x > 0.5 {
        linear = input.color;
    }
    var color = linear * light;
    // egui may use a non-sRGB target; encode once, according to the actual target.
    if camera.options.y > 0.5 {
        color = select(1.055 * pow(color, vec3<f32>(1.0 / 2.4)) - 0.055,
                       color * 12.92, color <= vec3<f32>(0.0031308));
    }
    return vec4<f32>(color, 1.0);
}
