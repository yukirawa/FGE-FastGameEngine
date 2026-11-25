@group(0) @binding(0)
var<uniform> camera: mat4x4<f32>;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(1) @binding(1)
var s_diffuse: sampler;

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_rect: vec4<f32>,
    @location(3) rotation: f32,
    @location(4) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) v_idx: u32,
    instance: InstanceInput,
) -> VertexOutput {
    let u_bit = f32(v_idx & 1u);
    let v_bit = f32((v_idx >> 1u) & 1u);
    let corner = vec2<f32>(u_bit - 0.5, v_bit - 0.5);

    let local_pos = corner * instance.size;
    let c = cos(instance.rotation);
    let s = sin(instance.rotation);
    let rotated_pos = vec2<f32>(
        local_pos.x * c - local_pos.y * s,
        local_pos.x * s + local_pos.y * c,
    );
    let world_pos = rotated_pos + instance.pos;

    var out: VertexOutput;
    out.clip_position = camera * vec4<f32>(world_pos, 0.0, 1.0);
    let is_full_uv = instance.uv_rect.x == 0.0
        && instance.uv_rect.y == 0.0
        && instance.uv_rect.z > 0.999
        && instance.uv_rect.w > 0.999;
    if (is_full_uv) {
        out.uv = vec2<f32>(4.0 / 2048.0, 4.0 / 2048.0);
    } else {
        out.uv = mix(instance.uv_rect.xy, instance.uv_rect.zw, vec2<f32>(u_bit, v_bit));
    }
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color * textureSample(t_diffuse, s_diffuse, in.uv);
}
