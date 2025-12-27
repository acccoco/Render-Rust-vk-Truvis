use truvis_app::outer_app::triangle::triangle_app::HelloTriangle;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::new(HelloTriangle::default());
    WinitApp::run(outer_app);
}
