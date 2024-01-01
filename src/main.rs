mod app;
mod graphic;
mod gui;

fn main() {
    pollster::block_on(app::start());
}
