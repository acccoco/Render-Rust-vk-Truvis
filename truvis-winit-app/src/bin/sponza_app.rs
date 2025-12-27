use truvis_app::outer_app::sponza_app::SponzaApp;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::new(SponzaApp::default());
    WinitApp::run(outer_app);
}
