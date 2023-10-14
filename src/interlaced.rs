use std::{rc::Rc, cell::RefCell};

use wgpu::util::DeviceExt;

use crate::utils::*;

pub struct InterlacedRendererState {
    /// Full width of the rendered frame.
    width: u32,
    /// Full height of the rendered frame.
    height: u32,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    render_texture1: wgpu::Texture,
    render_texture2: wgpu::Texture,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    need_write_data: bool,
    frame_number: u64,
    index_buffer: wgpu::Buffer,
}

struct UniformData {
    width: u32,
    height: u32,
}

impl InterlacedRendererState {
    /// Create a new interlaced renderer with an existing device.
    pub fn new(device: Rc<wgpu::Device>, queue: Rc<wgpu::Queue>, width: u32, height: u32, target: wgpu::TextureFormat, internal_shader_src: &str) -> Self {
        let uniform_data = UniformData {
            width,
            height,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Interlaced renderer uniform buffer"),
            contents: unsafe { any_as_u8_slice(&uniform_data) },
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // For render to texture, we use RENDER_ATTACHMENT to allow rendering to this texture, and TEXTURE_BINDING to allow reading it in another pass
        let render_texture1 = create_texture(device.as_ref(), Some("Interlaced renderer first render texture"), width, height / 2, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        let render_texture2 = create_texture(device.as_ref(), Some("Interlaced renderer second render texture"), width, height / 2, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        let render_view1 = render_texture1.create_view(&wgpu::TextureViewDescriptor::default());
        let render_view2 = render_texture2.create_view(&wgpu::TextureViewDescriptor::default());

        // let sampler = device.create_sampler(
        //     &wgpu::SamplerDescriptor {
        //         address_mode_u: wgpu::AddressMode::Repeat,
        //         address_mode_v: wgpu::AddressMode::Repeat,
        //         address_mode_w: wgpu::AddressMode::Repeat,
        //         mag_filter: wgpu::FilterMode::Linear,
        //         min_filter: wgpu::FilterMode::Nearest,
        //         mipmap_filter: wgpu::FilterMode::Linear,
        //         ..Default::default()
        //     }
        // );

        let bind_group_layout = create_bind_group_layout(&device, Some("Interlaced renderer bind group layout"), 
            vec![
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true }
                },
                wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true }
                },
                // wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            ], 
        wgpu::ShaderStages::FRAGMENT);

        let bind_group = create_bind_group(&device, Some("Interlaced renderer bind group"), &bind_group_layout, 
            vec![
                uniform_buffer.as_entire_binding(),
                wgpu::BindingResource::TextureView(&render_view1),
                wgpu::BindingResource::TextureView(&render_view2),
                // wgpu::BindingResource::Sampler(&sampler),
            ],
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Interlaced renderer shader"),
            source: wgpu::ShaderSource::Wgsl(internal_shader_src.into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Interlaced renderer pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = create_render_pipeline(device.as_ref(), None, &[], &render_pipeline_layout, &shader, target);

        let indices: &[u16; 6] = &[
            0, 1, 2,
            2, 1, 3,
        ];

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: unsafe { any_as_u8_slice(indices) },
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Self {
            width,
            height,
            device,
            queue,
            render_texture1,
            render_texture2,
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            need_write_data: false,
            frame_number: 0,
            index_buffer,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        println!("interlaced renderer resize to {}x{}", width, height);
        self.width = width;
        self.height = height;
        self.render_texture1 = create_texture(self.device.as_ref(), None, self.width, self.height / 2, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        self.render_texture2 = create_texture(self.device.as_ref(), None, self.width, self.height / 2, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        self.need_write_data = true;
    }

    /// Send necessary data to the GPU
    pub fn write_needed_data(&mut self) {
        if self.need_write_data {
            self.queue.write_buffer(&self.uniform_buffer, 0, unsafe { any_as_u8_slice(&UniformData { width: self.width, height: self.height }) });

            self.bind_group = create_bind_group(&self.device, Some("Interlaced renderer bind group"), &self.bind_group_layout, 
                vec![
                    self.uniform_buffer.as_entire_binding(),
                    wgpu::BindingResource::TextureView(&self.render_texture1.create_view(&Default::default())),
                    wgpu::BindingResource::TextureView(&self.render_texture2.create_view(&Default::default())),
                    // wgpu::BindingResource::Sampler(&sampler),
                ],
            );

            self.need_write_data = false;
        }
    }

    pub fn get_render_texture(&self) -> &wgpu::Texture {
        if (self.frame_number & 1) == 0 { &self.render_texture1 } else { &self.render_texture2 }
    }

    /// Returns the command buffer necessary to render a full frame (by interlacing new frame with the old one) to a given texture.
    pub fn draw(&mut self, output_view: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Interlaced renderer Encoder"),
        });

        {
            // render to the full resolution texture given by caller, by interlacing new_half_frame and self.render_texture
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Interlaced renderer pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
    
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        let command_buffer = encoder.finish();

        self.write_needed_data();
        self.frame_number += 1;
        self.queue.submit(std::iter::once(command_buffer));
    }
}