use std::rc::Rc;

use wgpu::{util::DeviceExt, ShaderStages};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    window::Window,
};

use test_wgpu::interlaced::InterlacedRendererState;

struct State {
    surface: wgpu::Surface,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    clear_color: wgpu::Color,
    render_pipeline: wgpu::RenderPipeline,
    interlaced_renderer: InterlacedRendererState,
    uniform_buffer: wgpu::Buffer,
    mouse_pos: [f32; 3],
    frame_number: u64,
    bind_group: wgpu::BindGroup,
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
            present_mode: test_wgpu::utils::select_prefered_presentmode(&surface_caps.present_modes, &vec![wgpu::PresentMode::Mailbox, wgpu::PresentMode::Fifo]).unwrap(),
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

        let mouse_pos = [0.0, 0.0, 0.0, 0.0]; // [3] is to tell shader code whether we need to draw mouse circle or not. The rest is useless but WGPU requires buffer size to be a power of 2.

        let uniform_data = MyUniform { mouse_pos, frame_number: 0, width: size.width, height: size.height };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: unsafe { test_wgpu::utils::any_as_u8_slice(&uniform_data) },
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = test_wgpu::utils::create_bind_group_layout(&device, Some("bind_group_layout"), vec![wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }], wgpu::ShaderStages::FRAGMENT);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout], // the final pass uses the texture as input to produce the on-screen image with some transform
            push_constant_ranges: &[],
        });

        let render_pipeline = test_wgpu::utils::create_render_pipeline(&device, None, &[], &render_pipeline_layout, &shader, wgpu::TextureFormat::Rgba8Unorm);

        let device_rc = Rc::new(device);
        let queue_rc = Rc::new(queue);
        let interlaced_renderer = InterlacedRendererState::new(device_rc.clone(), queue_rc.clone(), size.width, size.height, config.format, include_str!("shaders/merge.wgsl"));

        Self {
            window,
            surface,
            device: device_rc,
            queue: queue_rc,
            config,
            size,
            clear_color,
            render_pipeline,
            interlaced_renderer,
            uniform_buffer,
            frame_number: 0,
            mouse_pos: [mouse_pos[0], mouse_pos[1], mouse_pos[2]],
            bind_group,
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
            self.interlaced_renderer.resize(new_size.width, new_size.height);
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

        self.queue.write_buffer(&self.uniform_buffer, 0, unsafe { test_wgpu::utils::any_as_u8_slice(&uniform_data) });

        self.frame_number += 1;
    }

    fn render_to_texture(&self, view: &wgpu::TextureView) {
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
    
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Step 1: render a half frame
        let render_texture = self.interlaced_renderer.get_render_texture();
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render_to_texture(&render_view);

        // Step 2: render a full frame by using the last rendered frame combined with the previous frame saved internally by the interlaced renderer. That means the very first frame will be half black.
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.interlaced_renderer.draw(&view);

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
                        // let render_time = start.elapsed().as_nanos() as f32 / 1000000f32;
                        // println!("frame render time: {} ms, fps: {}", render_time, 1000.0 / render_time);
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
