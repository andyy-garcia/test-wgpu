struct GlobalUniform {
    mouse_pos: vec3<f32>,
    padding1: u32,
    frame_number_low: u32,
    frame_number_high: u32,
    viewport_width: u32,
    viewport_height: u32,
};

@group(0) @binding(0)
var<uniform> global: GlobalUniform;

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

fn ycoord_to_norm(f: f32) -> f32 {
    // scale [-1; 1] to [0; 1]
    return (f + 1.0) / 2.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4<f32>(in.vert_pos.x, in.vert_pos.y, 1.0, 1.0);

    var a = 1u;
    // alternate, each frame, which y-line will be discarded (dark)
    // if ((global.frame_number_low & 1u) == 1u) {
    //     a = 0u;
    // }

    if ((u32(f32(global.viewport_height) * ycoord_to_norm(in.vert_pos.y)) % 2u) == a) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let p = in.vert_pos.xy * 2.0;
    let m = global.mouse_pos.xy * 2.0;

    var d = sd_circle(p, 0.5);

    var col = vec3<f32>(0.9, 0.6, 0.3);

    if d <= 0.0 {
        col = vec3<f32>(0.65, 0.85, 1.0);
    }

    col *= 1.0 - exp(-6.0 * abs(d));
    col *= 0.8 + 0.2*cos(150.0*d);
    col = mix(col, vec3<f32>(1.0), 1.0 - smoothstep(0.0, 0.01, abs(d)));

    if global.mouse_pos.z > 1.0 {
        d = sd_circle(m, 0.5);
        col = mix(col, vec3<f32>(1.0, 1.0, 0.0), 1.0 - smoothstep(0.0, 0.005, abs(length(p - m) - abs(d)) - 0.0025));
        col = mix(col, vec3<f32>(1.0, 1.0, 0.0), 1.0 - smoothstep(0.0, 0.005, length(p - m) - 0.015));
    }

    // if ((u32(f32(global.viewport_height) * ycoord_to_norm(in.vert_pos.y)) & 1u) == 1u) {
    //     col.r /= 2.0;
    //     col.g /= 2.0;
    //     col.b /= 2.0;
    // }

    return vec4<f32>(col, 1.0);
}
 