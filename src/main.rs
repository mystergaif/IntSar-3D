// IntSar-3D: A Simple 3D Engine in Rust

// Module declarations
mod math;
mod renderer;
mod scene;

use winit::event_loop::EventLoop;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init();

    // Create event loop
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    // Create renderer
    let renderer = renderer::Renderer::new(&event_loop).await;

    // Run the renderer
    renderer.run(event_loop);
}
