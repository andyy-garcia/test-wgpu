use std::{rc::Rc, cell::RefCell};

/// Select the first desired present mode to be supported (among given lists)
pub fn select_prefered_presentmode(supported_modes: &Vec<wgpu::PresentMode>, desired_modes: &Vec<wgpu::PresentMode>) -> Option::<wgpu::PresentMode> {
    let mut desired_modes_iter = desired_modes.into_iter();
    let mut selected_mode  = Option::<wgpu::PresentMode>::None;

    loop {
        selected_mode = desired_modes_iter.next().copied();

        match selected_mode {
            Some(x) if supported_modes.iter().position(|&mode| mode == x) == None => {
                selected_mode = None;
            },
            _ => {
                break;
            }
        }
    }

    selected_mode
}

/// Create a simple 2D texture with Rgba8Unorm format (no multisampling, no mip-levels)
pub fn create_texture(device: &wgpu::Device, label: Option<&str>, width: u32, height: u32, usage: wgpu::TextureUsages) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d { width: width, height: height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: usage,
        label: label,
        view_formats: &[]
    })
}

/// Create a render pipeline with "vs_main" as vertex shader entry point, and "fs_main" AS fragment shader entry point, and some other default parameters. No multisampling.
pub fn create_render_pipeline(device: &wgpu::Device, label: Option<&str>, vertex_buffers: &[wgpu::VertexBufferLayout], pipeline_layout: &wgpu::PipelineLayout, shader_module: &wgpu::ShaderModule, target: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: label,
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader_module,
            entry_point: "vs_main",
            buffers: vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: target,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}

/// Create a bind group layout from a vector of bindings, with automatic binding indexes, and same visibility for every binding.
pub fn create_bind_group_layout(device: &wgpu::Device, label: Option<&str>, bindings: Vec<wgpu::BindingType>, global_visiblity: wgpu::ShaderStages) -> wgpu::BindGroupLayout {
    let mut entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];
    let mut counter = 0;

    for binding in bindings {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: counter,
            visibility: global_visiblity,
            ty: binding,
            count: None
        });

        counter += 1;
    }

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &entries,
        label: label,
    })
}

pub fn create_bind_group(device: &wgpu::Device, label: Option<&str>, layout: &wgpu::BindGroupLayout, resources: Vec<wgpu::BindingResource>) -> wgpu::BindGroup {
    let mut entries: Vec<wgpu::BindGroupEntry> = vec![];
    let mut counter = 0;

    for resource in resources {
        entries.push(wgpu::BindGroupEntry {
            binding: counter,
            resource: resource,
        });

        counter += 1;
    }

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: layout,
        entries: &entries,
        label: label,
    })
}

pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    core::slice::from_raw_parts(
        (p as *const T) as *const u8,
        core::mem::size_of::<T>(),
    )
}