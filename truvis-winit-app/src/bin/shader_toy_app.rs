use truvis_app::outer_app::shader_toy::shader_toy_app::ShaderToy;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::new(ShaderToy::default());
    WinitApp::run(outer_app);
}
