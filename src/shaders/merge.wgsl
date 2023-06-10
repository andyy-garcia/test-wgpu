// Common utility functions

fn coord_to_norm(f: f32) -> f32 {
    // scale [-1; 1] to [0; 1]
    return (f + 1.0) / 2.0;
}

fn norm_to_coord(f: f32) -> f32 {
    // scale [0; 1] to [-1; 1]
    return (f * 2.0) - 1.0;
}


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
    let x = norm_to_coord(f32(in_vertex_index & 1u));
    let y = norm_to_coord(f32(in_vertex_index & 2u));
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.vert_pos = out.clip_position.xyz;
    return out;
}


// Fragment shader

struct GlobalUniform {
    width: u32,
    height: u32,
};

@group(0) @binding(0)
var<uniform> global: GlobalUniform;

@group(0) @binding(1)
var input_texture1: texture_2d<f32>;

@group(0) @binding(2)
var input_texture2: texture_2d<f32>;

// @group(0) @binding(3)
// var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let x = u32(f32(global.width) * coord_to_norm(in.vert_pos.x));
    let y = u32(f32(global.height) * coord_to_norm(-in.vert_pos.y));

    var col1 = textureLoad(input_texture1, vec2<i32>(i32(x / 1u), i32(y / 2u)), 0);
    var col2 = textureLoad(input_texture2, vec2<i32>(i32(x / 1u), i32(y / 2u)), 0);

    var col = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    if ((y & 1u) == 0u) {
        col = col1;
        // return textureSampleLevel(input_texture1, texture_sampler, vec2<f32>(coord_to_norm(in.vert_pos.x), coord_to_norm(-in.vert_pos.y)), 1.0);
    } else {
        col = col2;
        // return textureSampleLevel(input_texture2, texture_sampler, vec2<f32>(coord_to_norm(in.vert_pos.x), coord_to_norm(-in.vert_pos.y)), 1.0);
    }

    return col;
}