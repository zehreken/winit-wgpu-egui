use wgpu::{Device, Queue, Surface, SurfaceConfiguration, TextureFormat};
use winit::{
    dpi::{PhysicalSize, Size},
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{graphic, gui};

pub struct App {
    window: Window,
    surface: Surface,
    device: Device,
    queue: Queue,
    size: PhysicalSize<u32>,
    surface_config: SurfaceConfiguration,
    texture_format: TextureFormat,
}

impl App {
    async fn new(window: Window) -> App {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Since app owns the window, this is safe
        // App's lifetime is longer than surface
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        // Async is fine but you can also use pollster::block_on without await
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        // Same with this one, pollster::block_on(adapter_request(...)).unwrap(); is another way
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let texture_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: size.width as u32,
            height: size.height as u32,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        Self {
            window,
            surface,
            device,
            queue,
            size,
            surface_config,
            texture_format,
        }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}

pub async fn start() {
    let size = Size::Physical(PhysicalSize {
        width: 1600,
        height: 1200,
    });
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(false)
        .with_transparent(false)
        .with_title("winit-wgpu-egui")
        .with_inner_size(size)
        .build(&event_loop)
        .unwrap();

    let app = App::new(window).await;
    // create graphic
    let graphic = graphic::Graphic::new(&app.device, &app.surface_config);
    // create gui
    let mut gui = gui::Gui::new(&event_loop, &app.device, app.texture_format);

    event_loop.run(move |event, _elwt, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
        } if window_id == app.window.id() => control_flow.set_exit(),
        Event::WindowEvent { event, .. } => {
            gui.handle_event(&event);
        }
        Event::MainEventsCleared => app.window.request_redraw(),
        Event::RedrawRequested(_) => {
            let output_frame = match app.surface.get_current_texture() {
                Ok(frame) => frame,
                Err(wgpu::SurfaceError::Outdated) => {
                    // This error occurs when the app is minimized on Windows.
                    // Silently return here to prevent spamming the console with:
                    // "The underlying surface has changed, and therefore the swap chain must be updated"
                    return;
                }
                Err(e) => {
                    eprintln!("Dropped frame with error: {}", e);
                    return;
                }
            };
            let output_view = output_frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = app
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });
            graphic.render(&mut encoder, &output_view);
            gui.render(&mut encoder, &app.window, &output_view, &app);
            app.queue.submit(Some(encoder.finish()));
            output_frame.present();
        }
        _ => {}
    });
}
