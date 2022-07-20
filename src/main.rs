use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

fn main() {
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

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

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
