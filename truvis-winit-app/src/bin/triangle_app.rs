use truvis_app::outer_app::triangle::triangle_app::HelloTriangleApp;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::new(HelloTriangleApp::default());
    WinitApp::run(outer_app);
}
