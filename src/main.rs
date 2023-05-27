use wgpu::{util::DeviceExt};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    window::Window,
};

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    clear_color: wgpu::Color,
    render_pipeline: wgpu::RenderPipeline,
    render_pipeline2: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    mouse_pos: [f32; 3],
    frame_number: u64,
    mouse_pos_need_update: bool,
    bind_group: wgpu::BindGroup,
    render_texture_view: wgpu::TextureView,
}

fn fallback_select_presentmode(supported_modes: &Vec<wgpu::PresentMode>, desired_modes: &Vec<wgpu::PresentMode>) -> Option::<wgpu::PresentMode> {
    let mut desired_modes_iter = desired_modes.into_iter();
    let mut selected_mode  = Option::<wgpu::PresentMode>::None;

    loop {
        selected_mode = desired_modes_iter.next().copied();

        if let Some(x) = selected_mode {
            // for i in supported_modes {
            //     println!("{} (selected: {})", *i as i32, x as i32);
            // }
            if supported_modes.iter().position(|&mode| mode == x) == None {
                selected_mode = None;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    selected_mode
}

fn create_flat_texture(device: &wgpu::Device, width: u32, height: u32, usage: wgpu::TextureUsages) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d { width: width, height: height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: usage,
        label: None,
        view_formats: &[]
    })
}

pub fn create_simple_render_pipeline(pipeline_layout: &wgpu::PipelineLayout, device: &wgpu::Device, shader_module: &wgpu::ShaderModule, target: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader_module,
            entry_point: "vs_main",
            buffers: &[],
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
        depth_stencil: None, // 1.
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    })
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::core::mem::size_of::<T>(),
    )
}

struct MyUniform {
    mouse_pos: [f32; 4],
    frame_number: u64,
    width: u32,
    height: u32,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: fallback_select_presentmode(&surface_caps.present_modes, &vec![wgpu::PresentMode::Mailbox, wgpu::PresentMode::Fifo]).unwrap(),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let mouse_pos = [0.0, 0.0, 0.0, 0.0]; // [3] is to tell shader code whether we need to draw mouse circle or not. The rest is useless but WGPU requires buffer size to be power-of-2-aligned.

        let uniform_data = MyUniform { mouse_pos, frame_number: 0, width: size.width, height: size.height };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: unsafe { any_as_u8_slice(&uniform_data) },
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("bind_group_layout"),
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader2.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = create_simple_render_pipeline(&render_pipeline_layout, &device, &shader, config.format);
        let render_pipeline2 = create_simple_render_pipeline(&render_pipeline_layout, &device, &shader, wgpu::TextureFormat::Rgba8Unorm);

        // For render-to-texture:
        // We use RENDER_ATTACHMENT to allow rendering to this texture, and STORAGE_BINDING to allow reading it in another render pass (we can use TEXTURE_BINDING if we need a sampler)
        let render_texture = create_flat_texture(&device, size.width / 2, size.height / 2, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::STORAGE_BINDING);
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            clear_color,
            render_pipeline,
            render_pipeline2,
            uniform_buffer,
            frame_number: 0,
            mouse_pos: [mouse_pos[0], mouse_pos[1], mouse_pos[2]],
            mouse_pos_need_update: false,
            bind_group,
            render_texture_view: render_view,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos[0] = (position.x / (self.size.width as f64)) as f32;
                self.mouse_pos[1] = (position.y / (self.size.height as f64)) as f32;

                true
            },
            WindowEvent::MouseInput { 
                state: ElementState::Pressed,
                ..
            } => {
                self.mouse_pos[2] = 2.0;
                false
            },
            WindowEvent::MouseInput { 
                state: ElementState::Released,
                .. 
            } => {
                self.mouse_pos[2] = 0.0;
                false
            }
            _ => {
                false
            }
        }
    }

    fn update(&mut self) {
        let mouse_pressed = self.mouse_pos[2] > 1.0;
        let must_update = self.mouse_pos_need_update || mouse_pressed;

        if must_update {
            // mouse position data is [0; 1] but shader use the [-1; 1] format (with Y being 1 at top and -1 at bottom).
            let mut mouse_pos = self.mouse_pos.clone();
            mouse_pos[0] *= 2.0;
            mouse_pos[0] -= 1.0;
            mouse_pos[1] *= 2.0;
            mouse_pos[1] -= 1.0;
            mouse_pos[1] = -mouse_pos[1];

            let uniform_data = MyUniform {
                mouse_pos: [mouse_pos[0], mouse_pos[1], mouse_pos[2], 0.0],
                frame_number: self.frame_number,
                height: self.size.height,
                width: self.size.width,
            };

            // println!("{:?}", unsafe { &any_as_u8_slice(&uniform_data) });

            self.queue.write_buffer(&self.uniform_buffer, 0, unsafe { &any_as_u8_slice(&uniform_data) });
            self.frame_number = self.frame_number + 1;

            if !self.mouse_pos_need_update {
                self.mouse_pos_need_update = true;
            } else if !mouse_pressed {
                self.mouse_pos_need_update = false;
            }
        }
    }

    fn render_to_texture(&self, view: &wgpu::TextureView, pipeline: &wgpu::RenderPipeline) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
    
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Step 1: offscreen rendering to a texture
        self.render_to_texture(&self.render_texture_view, &self.render_pipeline2);

        // Step 2: render on screen[, using the previously rendered texture as input in shader. => not done yet]
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render_to_texture(&view, &self.render_pipeline);
        
        output.present();
        Ok(())
    }    
}

async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        // new_inner_size is &&mut so we have to dereference it twice
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update();

                let start = std::time::Instant::now();
                match state.render() {
                    Ok(_) => {
                        let render_time = start.elapsed().as_nanos() as f32 / 1000000f32;
                        println!("frame render time: {} ms, fps: {}", render_time, 1000.0 / render_time);
                    }
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                state.window().request_redraw();
            }
            _ => {}
        };
    });
}

fn main() {
    pollster::block_on(run());
}
