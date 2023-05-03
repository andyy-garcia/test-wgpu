struct MouseUniform {
    mouse_pos: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> mouse: MouseUniform;

// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) vert_pos: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index));
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.vert_pos = out.clip_position.xyz;
    return out;
}

// Fragment shader

fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4<f32>(in.vert_pos.x, in.vert_pos.y, 1.0, 1.0);
    let p = in.vert_pos.xy * 2.0;
    let m = mouse.mouse_pos.xy * 2.0;
    var d = sd_circle(p, 0.5);

    var col = vec3<f32>(0.9, 0.6, 0.3);

    if d <= 0.0 {
        col = vec3<f32>(0.65, 0.85, 1.0);
    }

    col *= 1.0 - exp(-6.0 * abs(d));
    col *= 0.8 + 0.2*cos(150.0*d);
	col = mix(col, vec3<f32>(1.0), 1.0 - smoothstep(0.0, 0.01, abs(d)));

    if mouse.mouse_pos.z > 1.0 {
        d = sd_circle(m, 0.5);
        col = mix(col, vec3<f32>(1.0, 1.0, 0.0), 1.0 - smoothstep(0.0, 0.005, abs(length(p - m) - abs(d)) - 0.0025));
        col = mix(col, vec3<f32>(1.0, 1.0, 0.0), 1.0 - smoothstep(0.0, 0.005, length(p - m) - 0.015));
    }

    return vec4<f32>(col, 1.0);
}
 