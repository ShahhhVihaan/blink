use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, DeviceEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use wgpu::{Instance, Device, Queue, SurfaceConfiguration, util::DeviceExt};
use glam::{Vec3, Mat4, Quat};

#[derive(Default)]
struct App {
    window: Option<Window>,
    instance: Option<Instance>,
    device: Option<Device>,
    queue: Option<Queue>,
    config: Option<SurfaceConfiguration>,
    camera: Camera,
    render_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    uniform_buffer: Option<wgpu::Buffer>,
    uniform_bind_group: Option<wgpu::BindGroup>,
}

#[derive(Debug)]
struct Camera {
    position: Vec3,
    rotation: Quat,
    fov: f32,
    aspect: f32,
    near: f32,
    far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            rotation: Quat::IDENTITY,
            fov: 45.0_f32.to_radians(),
            aspect: 1.0,
            near: 0.1,
            far: 100.0,
        }
    }
}

impl Camera {
    fn view_matrix(&self) -> Mat4 {
        Mat4::from_translation(-self.position) * Mat4::from_quat(self.rotation)
    }

    fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop.create_window(Window::default_attributes()).unwrap();
        self.window = Some(window);
        
        // Initialize graphics
        self.init_graphics();
        
        // Request initial redraw
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            },
            WindowEvent::RedrawRequested => {
                self.render();
                // Request continuous redraws
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(physical_size) => {
                if let (Some(device), Some(instance), Some(config)) = 
                    (&self.device, &self.instance, &mut self.config) {
                    config.width = physical_size.width;
                    config.height = physical_size.height;
                    let window = self.window.as_ref().unwrap();
                    let surface = instance.create_surface(window).unwrap();
                    surface.configure(device, config);
                    self.camera.aspect = physical_size.width as f32 / physical_size.height as f32;
                }
                // Request redraw after resize
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: winit::event::DeviceId, event: DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                // Simple camera rotation with mouse
                if let Some(_window) = &self.window {
                    // For now, always allow camera movement
                    {
                        let sensitivity = 0.01;
                        let delta_x = delta.0 as f32 * sensitivity;
                        let delta_y = delta.1 as f32 * sensitivity;
                        
                        let rot_y = Quat::from_axis_angle(Vec3::Y, delta_x);
                        let rot_x = Quat::from_axis_angle(Vec3::X, delta_y);
                        self.camera.rotation = rot_y * rot_x * self.camera.rotation;
                    }
                }
            }
            _ => (),
        }
    }
}

impl App {
    fn init_graphics(&mut self) {
        let window = self.window.as_ref().unwrap();
        
        // Create instance
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        // Create surface
        let surface =   instance.create_surface(window).unwrap();

        // Get adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).unwrap();

        // Create device and queue
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        )).unwrap();

        self.device = Some(device);
        self.queue = Some(queue);
        self.instance = Some(instance);

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&self.device.as_ref().unwrap(), &config);
        self.config = Some(config);

        // Create shaders
        let shader = self.device.as_ref().unwrap().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create render pipeline
        let render_pipeline_layout = self.device.as_ref().unwrap().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&self.create_bind_group_layout()],
            push_constant_ranges: &[],
        });

        let render_pipeline = self.device.as_ref().unwrap().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        self.render_pipeline = Some(render_pipeline);

        // Create vertex buffer (simple cube)
        let vertices = create_cube_vertices();
        let vertex_buffer = self.device.as_ref().unwrap().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.vertex_buffer = Some(vertex_buffer);

        // Create index buffer
        let indices = create_cube_indices();
        let index_buffer = self.device.as_ref().unwrap().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.index_buffer = Some(index_buffer);

        // Create uniform buffer and bind group
        let uniform_buffer = self.device.as_ref().unwrap().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                view_proj: (self.camera.projection_matrix() * self.camera.view_matrix()).to_cols_array_2d(),
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        self.uniform_buffer = Some(uniform_buffer);

        let bind_group = self.device.as_ref().unwrap().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.create_bind_group_layout(),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.uniform_buffer.as_ref().unwrap().as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });
        self.uniform_bind_group = Some(bind_group);
    }

    fn create_bind_group_layout(&self) -> wgpu::BindGroupLayout {
        self.device.as_ref().unwrap().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        })
    }

    fn render(&mut self) {
        if let (Some(device), Some(instance), Some(queue), Some(config), Some(pipeline), Some(vertex_buffer), Some(index_buffer), Some(uniform_bind_group)) = 
            (&self.device, &self.instance, &self.queue, &self.config, &self.render_pipeline, &self.vertex_buffer, &self.index_buffer, &self.uniform_bind_group) {
            
            let window = self.window.as_ref().unwrap();
            let surface =  instance.create_surface(window) .unwrap();
            surface.configure(device, config);
            let frame = surface.get_current_texture().unwrap();
            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
            
            // Update uniforms
            let uniforms = Uniforms {
                view_proj: (self.camera.projection_matrix() * self.camera.view_matrix()).to_cols_array_2d(),
            };
            queue.write_buffer(&self.uniform_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&[uniforms]));

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..36, 0, 0..1);
            }

            queue.submit(std::iter::once(encoder.finish()));
            frame.present();
        }
    }
}

fn create_cube_vertices() -> Vec<Vertex> {
    vec![
        // Front face
        Vertex { position: [-1.0, -1.0,  1.0], color: [1.0, 0.0, 0.0] },
        Vertex { position: [ 1.0, -1.0,  1.0], color: [0.0, 1.0, 0.0] },
        Vertex { position: [ 1.0,  1.0,  1.0], color: [0.0, 0.0, 1.0] },
        Vertex { position: [-1.0,  1.0,  1.0], color: [1.0, 1.0, 0.0] },
        // Back face
        Vertex { position: [-1.0, -1.0, -1.0], color: [1.0, 0.0, 1.0] },
        Vertex { position: [-1.0,  1.0, -1.0], color: [0.0, 1.0, 1.0] },
        Vertex { position: [ 1.0,  1.0, -1.0], color: [1.0, 1.0, 1.0] },
        Vertex { position: [ 1.0, -1.0, -1.0], color: [0.5, 0.5, 0.5] },
    ]
}

fn create_cube_indices() -> Vec<u16> {
    vec![
        0, 1, 2,  2, 3, 0,  // front
        4, 5, 6,  6, 7, 4,  // back
        0, 4, 7,  7, 1, 0,  // bottom
        2, 6, 5,  5, 3, 2,  // top
        0, 3, 5,  5, 4, 0,  // left
        1, 7, 6,  6, 2, 1,  // right
    ]
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}