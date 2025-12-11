// Renderer module for IntSar-3D

use winit::{
    event::{Event, WindowEvent, KeyEvent, ElementState},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    keyboard::{PhysicalKey, KeyCode},
};
use wgpu::{Adapter, Instance, RenderPipeline, Buffer}; // Import necessary types
use std::sync::Arc;
use glam::{Mat4, Vec3, Quat};
use std::time::Instant;

pub struct Renderer {
    instance: Instance,
    adapter: Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    window: Arc<winit::window::Window>,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    uniform_buffer: Buffer,
    uniform_bind_group: wgpu::BindGroup,
    surface_format: wgpu::TextureFormat,
    camera_position: Vec3,
    cube_rotation: Vec3,
    start_time: Instant,
    keys_pressed: KeyboardState,
}

#[derive(Default)]
struct KeyboardState {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
}

// Define Vertex struct for vertex data
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

// Uniform buffer structure for MVP matrix
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
}

impl Uniforms {
    fn new() -> Self {
        Self {
            mvp: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update_mvp(&mut self, mvp: Mat4) {
        self.mvp = mvp.to_cols_array_2d();
    }
}

impl Renderer {
    pub async fn new(event_loop: &EventLoop<()>) -> Self {
        // Create window with Arc for shared ownership
        let window = Arc::new(WindowBuilder::new()
            .with_title("IntSar-3D")
            .build(event_loop)
            .unwrap());

        // Initialize wgpu
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Get surface from window
        let surface = instance.create_surface(window.clone()).expect("Failed to create surface");

        // Request adapter
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Renderer Device"),
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
            },
            None, // Trace path
        ).await.unwrap();

        // Get surface capabilities
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        
        // Configure surface
        let size = window.inner_size();
        surface.configure(&device, &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        });

        // Load shader
        let shader_code = include_str!("shader.wgsl");
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        // Define vertex buffer layout
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
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
        };

        // Create bind group layout for uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
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

        // Create cube vertices with colors
        let vertices = [
            // Front face (red)
            Vertex { position: [-0.5, -0.5,  0.5], color: [1.0, 0.0, 0.0] },
            Vertex { position: [ 0.5, -0.5,  0.5], color: [1.0, 0.0, 0.0] },
            Vertex { position: [ 0.5,  0.5,  0.5], color: [1.0, 0.0, 0.0] },
            Vertex { position: [-0.5,  0.5,  0.5], color: [1.0, 0.0, 0.0] },
            
            // Back face (green)
            Vertex { position: [-0.5, -0.5, -0.5], color: [0.0, 1.0, 0.0] },
            Vertex { position: [-0.5,  0.5, -0.5], color: [0.0, 1.0, 0.0] },
            Vertex { position: [ 0.5,  0.5, -0.5], color: [0.0, 1.0, 0.0] },
            Vertex { position: [ 0.5, -0.5, -0.5], color: [0.0, 1.0, 0.0] },
            
            // Top face (blue)
            Vertex { position: [-0.5,  0.5, -0.5], color: [0.0, 0.0, 1.0] },
            Vertex { position: [-0.5,  0.5,  0.5], color: [0.0, 0.0, 1.0] },
            Vertex { position: [ 0.5,  0.5,  0.5], color: [0.0, 0.0, 1.0] },
            Vertex { position: [ 0.5,  0.5, -0.5], color: [0.0, 0.0, 1.0] },
            
            // Bottom face (yellow)
            Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 1.0, 0.0] },
            Vertex { position: [ 0.5, -0.5, -0.5], color: [1.0, 1.0, 0.0] },
            Vertex { position: [ 0.5, -0.5,  0.5], color: [1.0, 1.0, 0.0] },
            Vertex { position: [-0.5, -0.5,  0.5], color: [1.0, 1.0, 0.0] },
            
            // Right face (magenta)
            Vertex { position: [ 0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] },
            Vertex { position: [ 0.5,  0.5, -0.5], color: [1.0, 0.0, 1.0] },
            Vertex { position: [ 0.5,  0.5,  0.5], color: [1.0, 0.0, 1.0] },
            Vertex { position: [ 0.5, -0.5,  0.5], color: [1.0, 0.0, 1.0] },
            
            // Left face (cyan)
            Vertex { position: [-0.5, -0.5, -0.5], color: [0.0, 1.0, 1.0] },
            Vertex { position: [-0.5, -0.5,  0.5], color: [0.0, 1.0, 1.0] },
            Vertex { position: [-0.5,  0.5,  0.5], color: [0.0, 1.0, 1.0] },
            Vertex { position: [-0.5,  0.5, -0.5], color: [0.0, 1.0, 1.0] },
        ];

        // Create indices for the cube
        #[rustfmt::skip]
        let indices: &[u16] = &[
            0,  1,  2,  2,  3,  0,  // front
            4,  5,  6,  6,  7,  4,  // back
            8,  9,  10, 10, 11, 8,  // top
            12, 13, 14, 14, 15, 12, // bottom
            16, 17, 18, 18, 19, 16, // right
            20, 21, 22, 22, 23, 20, // left
        ];

        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create uniform buffer
        let uniform_data = Uniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            window,
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            surface_format,
            camera_position: Vec3::new(0.0, 0.0, 3.0),
            cube_rotation: Vec3::ZERO,
            start_time: Instant::now(),
            keys_pressed: KeyboardState::default(),
        }
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        let _ = event_loop.run(move |event, target| {
            target.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::CloseRequested,
                } if window_id == self.window.id() => {
                    target.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(physical_size),
                    window_id,
                } if window_id == self.window.id() => {
                    self.resize(physical_size);
                }
                Event::AboutToWait => {
                    self.window.request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    window_id,
                } if window_id == self.window.id() => {
                    self.update_and_render();
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event, .. },
                    window_id,
                } if window_id == self.window.id() => {
                    self.handle_keyboard_input(event);
                }
                _ => {}
            }
        });
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        
        let surface_caps = self.surface.get_capabilities(&self.adapter);
            
        self.surface.configure(&self.device, &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            width: new_size.width,
            height: new_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        });
    }

    fn handle_keyboard_input(&mut self, event: KeyEvent) {
        if let PhysicalKey::Code(keycode) = event.physical_key {
            let is_pressed = event.state == ElementState::Pressed;
            match keycode {
                KeyCode::KeyW => self.keys_pressed.w = is_pressed,
                KeyCode::KeyA => self.keys_pressed.a = is_pressed,
                KeyCode::KeyS => self.keys_pressed.s = is_pressed,
                KeyCode::KeyD => self.keys_pressed.d = is_pressed,
                _ => {}
            }
        }
    }

    fn update_and_render(&mut self) {
        // Update cube rotation based on keyboard input
        let rotation_speed = 2.0 * 0.016; // Assuming ~60 FPS
        
        if self.keys_pressed.w {
            self.cube_rotation.x -= rotation_speed;
        }
        if self.keys_pressed.s {
            self.cube_rotation.x += rotation_speed;
        }
        if self.keys_pressed.a {
            self.cube_rotation.y -= rotation_speed;
        }
        if self.keys_pressed.d {
            self.cube_rotation.y += rotation_speed;
        }

        // Auto-rotate if no keys pressed
        let elapsed = self.start_time.elapsed().as_secs_f32();
        if !self.keys_pressed.w && !self.keys_pressed.a
           && !self.keys_pressed.s && !self.keys_pressed.d {
            self.cube_rotation.y = elapsed;
        }

        // Create transformation matrices
        let aspect_ratio = self.window.inner_size().width as f32
                         / self.window.inner_size().height as f32;
        
        // Model matrix (cube transformation)
        let model = Mat4::from_rotation_x(self.cube_rotation.x)
                  * Mat4::from_rotation_y(self.cube_rotation.y)
                  * Mat4::from_rotation_z(self.cube_rotation.z);
        
        // View matrix (camera)
        let view = Mat4::look_at_rh(
            self.camera_position,
            Vec3::ZERO,
            Vec3::Y,
        );
        
        // Projection matrix
        let projection = Mat4::perspective_rh_gl(
            45.0_f32.to_radians(),
            aspect_ratio,
            0.1,
            100.0,
        );
        
        // Combine into MVP matrix
        let mvp = projection * view * model;
        
        // Update uniform buffer
        let mut uniforms = Uniforms::new();
        uniforms.update_mvp(mvp);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        self.render();
    }

    fn render(&mut self) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                self.resize(self.window.inner_size());
                return;
            }
        };
        
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..36, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}