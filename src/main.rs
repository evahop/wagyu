use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

type Pos = [f32; 2];
type Col = [f32; 3];
struct Vert(Pos, Col);

#[repr(C, align(8))]
struct Size(u32, u32);

#[repr(C)]
struct Uniform {
    secs: f32,
    size: Size,
}

fn to_bytes<T>(t: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(t as *const _ as _, std::mem::size_of_val(t)) }
}

fn main() {
    let epoch = std::time::Instant::now();

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let (device, queue, surface, mut surface_config) = pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter
            .request_device(&Default::default(), None)
            .await
            .unwrap();

        let surface = unsafe { instance.create_surface(&window) };
        let (width, height) = window.inner_size().into();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *surface.get_supported_formats(&adapter).first().unwrap(),
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        (device, queue, surface, config)
    });

    let render_pipeline = {
        let shader_module = {
            let desc = wgpu::include_wgsl!("shader.wgsl");
            device.create_shader_module(desc)
        };
        let fragment_state = wgpu::FragmentState {
            module: &shader_module,
            entry_point: "frag_main",
            targets: &[Some(surface_config.format.into())],
        };
        let desc = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vert_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vert>() as _,
                    step_mode: Default::default(),
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: std::mem::size_of::<Pos>() as _,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 4,
                ..Default::default()
            },
            fragment: Some(fragment_state),
            multiview: None,
        };
        device.create_render_pipeline(&desc)
    };

    #[rustfmt::skip]
    let verts = &[
        Vert([-250.0,  250.0], [1.0, 0.0, 1.0]),
        Vert([ 250.0, -250.0], [0.0, 1.0, 1.0]),
        Vert([ 250.0,  250.0], [0.0, 0.0, 1.0]),
        Vert([-250.0,  250.0], [1.0, 0.0, 1.0]),
        Vert([-250.0, -250.0], [1.0, 1.0, 0.0]),
        Vert([ 250.0, -250.0], [0.0, 1.0, 1.0]),
    ];

    let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: to_bytes(verts),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<Uniform>() as _,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &render_pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(PhysicalSize { width, height }) => {
                    surface_config.width = width;
                    surface_config.height = height;
                    surface.configure(&device, &surface_config)
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                let uniform = Uniform {
                    secs: epoch.elapsed().as_secs_f32(),
                    size: Size(surface_config.width, surface_config.height),
                };
                queue.write_buffer(&uniform_buffer, 0, to_bytes(&uniform));

                let texture_view = {
                    let desc = wgpu::TextureDescriptor {
                        label: None,
                        size: wgpu::Extent3d {
                            width: surface_config.width,
                            height: surface_config.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 4,
                        dimension: wgpu::TextureDimension::D2,
                        format: surface_config.format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    };
                    device
                        .create_texture(&desc)
                        .create_view(&Default::default())
                };

                let surface_texture = surface.get_current_texture().unwrap();
                let resolve_target = surface_texture.texture.create_view(&Default::default());

                let mut command_encoder = device.create_command_encoder(&Default::default());
                {
                    let mut render_pass =
                        command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &texture_view,
                                resolve_target: Some(&resolve_target),
                                ops: Default::default(),
                            })],
                            ..Default::default()
                        });
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.set_pipeline(&render_pipeline);
                    render_pass.draw(0..verts.len() as _, 0..1)
                }

                queue.submit(Some(command_encoder.finish()));
                surface_texture.present()
            }
            _ => (),
        }
    })
}
