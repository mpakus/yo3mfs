struct Camera { view_projection: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec3<f32>,
           @location(1) col0: vec4<f32>, @location(2) col1: vec4<f32>,
           @location(3) col2: vec4<f32>, @location(4) col3: vec4<f32>) -> VertexOutput {
    let world = mat4x4<f32>(col0, col1, col2, col3) * vec4<f32>(position, 1.0);
    var out: VertexOutput;
    out.clip = camera.view_projection * world;
    out.world = world.xyz;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // World derivatives preserve flat faces under reflection and nonuniform scaling.
    let normal = normalize(cross(dpdx(input.world), dpdy(input.world)));
    let key = abs(dot(normal, normalize(vec3<f32>(-0.4, -0.7, 1.0))));
    let fill = abs(dot(normal, normalize(vec3<f32>(0.8, 0.3, 0.2))));
    let light = 0.32 + 0.5 * key + 0.18 * fill;
    return vec4<f32>(vec3<f32>(0.68, 0.76, 0.82) * light, 1.0);
}
