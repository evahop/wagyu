use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

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
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(fragment_state),
            multiview: None,
        };
        device.create_render_pipeline(&desc)
    };

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

                let surface_texture = surface.get_current_texture().unwrap();
                let texture_view = surface_texture.texture.create_view(&Default::default());

                let mut command_encoder = device.create_command_encoder(&Default::default());
                {
                    let mut render_pass =
                        command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &texture_view,
                                resolve_target: None,
                                ops: Default::default(),
                            })],
                            ..Default::default()
                        });
                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.set_pipeline(&render_pipeline);
                    render_pass.draw(0..3, 0..1)
                }

                queue.submit(Some(command_encoder.finish()));
                surface_texture.present()
            }
            _ => (),
        }
    })
}
