struct Camera {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    albedo: vec4<f32>,
    light_direction: vec4<f32>,
    light_color_intensity: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.view_proj * camera.model * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    out.normal = normalize((camera.model * vec4<f32>(input.normal, 0.0)).xyz);
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.normal);
    let l = normalize(-camera.light_direction.xyz);
    let diffuse = max(dot(n, l), 0.0);
    let ambient = 0.18;
    let lit = ambient + diffuse * camera.light_color_intensity.w;
    let light_color = camera.light_color_intensity.xyz;
    let base = input.color * camera.albedo.rgb;
    return vec4<f32>(base * (ambient + light_color * lit), camera.albedo.a);
}
