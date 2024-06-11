use std::collections::VecDeque;

use winit::{
    dpi::{LogicalSize, PhysicalSize, Size},
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{gui, renderer};

pub struct App {
    rolling_frame_time: VecDeque<f32>,
}

impl App {
    async fn new() -> App {
        let init = vec![0.0; 60];
        Self {
            rolling_frame_time: VecDeque::from(init),
        }
    }
}

pub async fn start() {
    let size = Size::Physical(PhysicalSize {
        width: 1600,
        height: 1200,
    });
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(false)
        .with_transparent(false)
        .with_title("winit-wgpu-egui")
        .with_inner_size(size)
        .build(&event_loop)
        .unwrap();

    let app = App::new().await;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
    }
    .unwrap();

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
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
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
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);
    // create renderer
    let mut renderer = renderer::Renderer::new(&device, &surface_config);
    // create gui
    let mut gui = gui::Gui::new(&window, &device, texture_format);

    let init = [0.0; 60];
    let mut rolling_frame_times = VecDeque::from(init);
    let mut earlier = std::time::Instant::now();
    let mut elapsed_time = 0.0;

    let r = event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
        } if window.id() == window_id => elwt.exit(),
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            let frame_time = std::time::Instant::now().duration_since(earlier);
            elapsed_time += frame_time.as_secs_f32();
            earlier = std::time::Instant::now();
            rolling_frame_times.pop_front();
            rolling_frame_times.push_back(frame_time.as_secs_f32());
            let fps = calculate_fps(&rolling_frame_times);
            let output_frame = match surface.get_current_texture() {
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

            renderer.render(&device, &queue, &output_view, elapsed_time);
            gui.render(&window, &output_view, &device, &queue, fps);
            output_frame.present();
            window.request_redraw();
        }
        Event::WindowEvent { event, .. } => {
            gui.handle_event(&window, &event);
        }
        _ => {}
    });
}

pub fn calculate_fps(times: &VecDeque<f32>) -> f32 {
    let sum: f32 = times.iter().sum();

    let average_time = sum / times.len() as f32;
    return 1.0 / average_time;
}
