mod app;
mod gui;
mod renderer;

fn main() {
    pollster::block_on(app::start());
}
