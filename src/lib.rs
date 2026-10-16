pub mod mesh;

use bytemuck::{Pod, Zeroable};
use cgmath::{Deg, Matrix4, Point3, Vector3};
use std::iter;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

/// Represents a single vertex in a mesh.
///
/// Contains a 4D position vector. The `w` component is utilized by shaders
/// to determine the type of object the vertex belongs to (e.g., terrain vs clouds).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub pos: [f32; 4],
}

impl Vertex {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Uniform buffer data passed to the shaders.
///
/// Contains the Model-View-Projection matrix and a time-based offset
/// used for animating the procedural generation.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct UniformBufferObject {
    pub mvp: [[f32; 4]; 4],
    pub offset: f32,
    pub padding: [f32; 3],
}

/// Holds the application and graphics state.
///
/// Contains all `wgpu` resources including the device, queue, surface,
/// rendering pipelines, buffers for terrain and clouds, and timing info.
struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    terrain_pipeline: wgpu::RenderPipeline,
    cloud_pipeline: wgpu::RenderPipeline,
    terrain_vertex_buffer: wgpu::Buffer,
    terrain_index_buffer: wgpu::Buffer,
    terrain_num_indices: u32,
    cloud_vertex_buffer: wgpu::Buffer,
    cloud_index_buffer: wgpu::Buffer,
    cloud_num_indices: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
    #[cfg(target_arch = "wasm32")]
    start_time_wasm: f64,
    #[cfg(not(target_arch = "wasm32"))]
    start_time: Instant,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    config_format: wgpu::TextureFormat,
    label: &str,
    blend_state: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(Vertex::desc())],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config_format,
                blend: Some(blend_state),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}

fn create_buffer<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: &str,
    contents: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(contents),
        usage,
    })
}

fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> (wgpu::Texture, wgpu::TextureView) {
    let size = wgpu::Extent3d {
        width: config.width.max(1),
        height: config.height.max(1),
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        #[cfg(target_arch = "wasm32")]
        let size = {
            use winit::platform::web::WindowExtWebSys;
            let canvas = window.canvas().unwrap();
            winit::dpi::PhysicalSize::new(canvas.width(), canvas.height())
        };
        #[cfg(not(target_arch = "wasm32"))]
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                )
            })
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: wgpu::SurfaceColorSpace::default(),
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let (_, depth_view) = create_depth_texture(&device, &config);

        let (terrain_vertices, terrain_indices) = mesh::generate_terrain();
        let (cloud_vertices, cloud_indices) = mesh::generate_clouds();

        use wgpu::util::DeviceExt;

        let terrain_vertex_buffer = create_buffer(
            &device,
            "Terrain Vertex Buffer",
            &terrain_vertices,
            wgpu::BufferUsages::VERTEX,
        );
        let terrain_index_buffer = create_buffer(
            &device,
            "Terrain Index Buffer",
            &terrain_indices,
            wgpu::BufferUsages::INDEX,
        );

        let cloud_vertex_buffer = create_buffer(
            &device,
            "Cloud Vertex Buffer",
            &cloud_vertices,
            wgpu::BufferUsages::VERTEX,
        );
        let cloud_index_buffer = create_buffer(
            &device,
            "Cloud Index Buffer",
            &cloud_indices,
            wgpu::BufferUsages::INDEX,
        );

        let initial_ubo = UniformBufferObject {
            mvp: [[0.0; 4]; 4],
            offset: 0.0,
            padding: [0.0; 3],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[initial_ubo]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let terrain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Terrain Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shader/terrain.wgsl").into()),
        });

        let cloud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cloud Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shader/cloud.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&uniform_bind_group_layout)],
                immediate_size: 0,
            });

        let terrain_pipeline = create_render_pipeline(
            &device,
            &render_pipeline_layout,
            &terrain_shader,
            config.format,
            "Terrain Pipeline",
            wgpu::BlendState::REPLACE,
        );

        let cloud_pipeline = create_render_pipeline(
            &device,
            &render_pipeline_layout,
            &cloud_shader,
            config.format,
            "Cloud Pipeline",
            wgpu::BlendState::ALPHA_BLENDING,
        );

        #[cfg(target_arch = "wasm32")]
        let start_time_wasm = web_sys::window().unwrap().performance().unwrap().now();

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            terrain_pipeline,
            cloud_pipeline,
            terrain_vertex_buffer,
            terrain_index_buffer,
            terrain_num_indices: terrain_indices.len() as u32,
            cloud_vertex_buffer,
            cloud_index_buffer,
            cloud_num_indices: cloud_indices.len() as u32,
            uniform_buffer,
            uniform_bind_group,

            depth_view,

            #[cfg(target_arch = "wasm32")]
            start_time_wasm,
            #[cfg(not(target_arch = "wasm32"))]
            start_time: Instant::now(),
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            let (_, depth_view) = create_depth_texture(&self.device, &self.config);
            self.depth_view = depth_view;
        }
    }

    fn update(&mut self) {
        #[cfg(target_arch = "wasm32")]
        let elapsed = ((web_sys::window().unwrap().performance().unwrap().now()
            - self.start_time_wasm)
            / 1000.0) as f32;

        #[cfg(not(target_arch = "wasm32"))]
        let elapsed = self.start_time.elapsed().as_secs_f32();

        let speed = 5.0; // Units per second
        let offset = -elapsed * speed;

        let window_width = self.config.width as f32;
        let window_height = self.config.height as f32;

        // Create the Model-View-Projection matrix
        let view = Matrix4::look_at_rh(
            // Eye: Position of the camera in world space
            Point3::new(0.0, 5.0, 0.0),
            // Center: Where the camera is looking
            Point3::new(0.0, 4.0, -10.0),
            // Up: Which way is "up" for the camera
            Vector3::new(0.0, 1.0, 0.0),
        );
        // The last two arguments set the near and far clipping planes
        let proj = cgmath::perspective(
            Deg(90.0),
            if window_height > 0.0 {
                window_width / window_height
            } else {
                1.0
            },
            0.1,
            500.0,
        );
        let model = Matrix4::from_translation(Vector3::new(0.0, -2.0, 0.0));
        let mvp = proj * view * model;

        let ubo = UniformBufferObject {
            mvp: mvp.into(),
            offset,
            padding: [0.0; 3],
        };

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[ubo]));
    }

    fn render(&mut self) {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                // The frame is still usable, but the surface should be reconfigured.
                self.resize(self.size);
                texture
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.resize(self.size);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => {
                log::warn!("Surface texture unavailable, skipping frame");
                return;
            }
            other => {
                log::warn!("Surface acquisition failed: {other:?}, skipping frame");
                return;
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
                            // Sky blue background
                            r: 0.3,
                            g: 0.5,
                            b: 0.8,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            render_pass.set_pipeline(&self.terrain_pipeline);
            render_pass.set_vertex_buffer(0, self.terrain_vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.terrain_index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..self.terrain_num_indices, 0, 0..1);

            render_pass.set_pipeline(&self.cloud_pipeline);
            render_pass.set_vertex_buffer(0, self.cloud_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.cloud_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.cloud_num_indices, 0, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        self.queue.present(output);
    }
}

/// Application state driving the `winit` event loop.
///
/// On native targets the GPU `State` is created synchronously during `resumed`.
/// On WebAssembly it is created asynchronously and delivered back through an
/// [`EventLoopProxy`] as a user event.
struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Mountain Flyby Demo")
            .with_inner_size(winit::dpi::LogicalSize::new(1920.0, 1080.0));
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let canvas = window.canvas()?;
                    // `Window::inner_size()` reports 0 until winit's ResizeObserver fires,
                    // so give the canvas an explicit bitmap size up front.
                    canvas.set_width(1920);
                    canvas.set_height(1080);
                    let dst = doc.get_element_by_id("wasm-example")?;
                    dst.append_child(&web_sys::Element::from(canvas)).ok()?;
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");

            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    let state = State::new(window).await;
                    assert!(proxy.send_event(state).is_ok());
                });
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State::new(window)));
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        event.resize(event.window.inner_size());
        event.window.request_redraw();
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.state else {
            return;
        };

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                state.resize(physical_size);
            }
            WindowEvent::RedrawRequested => {
                state.update();
                state.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

/// Initializes the application and runs the main event loop.
///
/// This sets up the window, logging, and starts the `winit` event loop
/// to handle rendering and window events.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::<State>::with_user_event().build().unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        let app = App {
            proxy: Some(event_loop.create_proxy()),
            state: None,
        };
        event_loop.spawn_app(app);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App { state: None };
        event_loop.run_app(&mut app).unwrap();
    }
}
