use truvis_app::outer_app::cornell_app;
use truvis_app::outer_app::cornell_app::CornellApp;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::new(CornellApp::default());
    WinitApp::run(outer_app);
}
