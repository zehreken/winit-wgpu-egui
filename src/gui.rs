use crate::app::App;
use egui::{Color32, RichText};
use egui_wgpu::wgpu::TextureFormat;
use egui_wgpu::{renderer::ScreenDescriptor, Renderer};
use egui_winit::{
    egui::{self, ClippedPrimitive, Context, TexturesDelta},
    State,
};
use wgpu::Device;
use winit::{event_loop::EventLoopWindowTarget, window::Window};

struct Test {
    is_window_open: bool,
}

impl Test {
    fn new() -> Self {
        Self {
            is_window_open: true,
        }
    }

    fn draw(&mut self, ctx: &Context, fps: f32) {
        egui::TopBottomPanel::top("menubar_container").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label(RichText::new(format!("FPS: {0:.2}", fps)).color(Color32::RED));
                ui.menu_button("File", |ui| {
                    if ui.button("About...").clicked() {
                        self.is_window_open = true;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::Window::new("Hello, winit-wgpu-egui")
            .open(&mut self.is_window_open)
            .show(ctx, |ui| {
                ui.label(
                    "This is the most basic example of how to use winit, wgpu and egui together.",
                );
                ui.label("Mandatory heart: ♥");

                ui.separator();

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x /= 2.0;
                    ui.label("Learn more about wgpu at");
                    ui.hyperlink("https://docs.rs/winit");
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x /= 2.0;
                    ui.label("Learn more about winit at");
                    ui.hyperlink("https://docs.rs/wgpu");
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x /= 2.0;
                    ui.label("Learn more about egui at");
                    ui.hyperlink("https://docs.rs/egui");
                });
            });
    }
}

pub struct Gui {
    ctx: Context,
    state: State,
    renderer: Renderer,
    screen_descriptor: ScreenDescriptor,
    view: Test,
    paint_jobs: Vec<ClippedPrimitive>,
    textures: TexturesDelta,
}

impl Gui {
    pub fn new<T>(
        event_loop: &EventLoopWindowTarget<T>,
        device: &wgpu::Device,
        texture_format: TextureFormat,
    ) -> Self {
        let scale_factor = 2.;
        let (width, height) = (1600, 1200);
        let max_texture_size = device.limits().max_texture_dimension_2d as usize;

        let egui_ctx = Context::default();
        let mut egui_state = egui_winit::State::new(event_loop);
        egui_state.set_max_texture_side(max_texture_size);
        egui_state.set_pixels_per_point(scale_factor);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: scale_factor,
        };
        let renderer = Renderer::new(device, texture_format, None, 1);
        let textures = TexturesDelta::default();

        let view = Test::new();

        Self {
            ctx: egui_ctx,
            state: egui_state,
            renderer,
            screen_descriptor,
            view,
            paint_jobs: vec![],
            textures,
        }
    }

    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        let _ = self.state.on_event(&self.ctx, event);
    }

    // resize

    // update scale factor

    pub fn render(
        &mut self,
        window: &Window,
        render_target: &wgpu::TextureView,
        app: &App,
        fps: f32,
    ) {
        let raw_input = self.state.take_egui_input(window);
        let output = self.ctx.run(raw_input, |egui_ctx| {
            self.view.draw(egui_ctx, fps);
        });

        self.textures.append(output.textures_delta);
        self.state
            .handle_platform_output(window, &self.ctx, output.platform_output);
        self.paint_jobs = self.ctx.tessellate(output.shapes);

        // Upload all resources to the GPU.
        for (id, image_delta) in &self.textures.set {
            self.renderer
                .update_texture(app.device(), app.queue(), *id, image_delta);
        }
        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gui encoder"),
            });
        self.renderer.update_buffers(
            app.device(),
            app.queue(),
            &mut encoder,
            &self.paint_jobs,
            &self.screen_descriptor,
        );

        // Render egui with WGPU
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.renderer
                .render(&mut rpass, &self.paint_jobs, &self.screen_descriptor);
        }
        // dropping rpass here
        app.queue().submit(Some(encoder.finish()));
        // Cleanup
        let textures = std::mem::take(&mut self.textures);
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
    }
}
