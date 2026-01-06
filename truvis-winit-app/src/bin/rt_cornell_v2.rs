use truvis_app::outer_app::cornell_app_v2::CornellAppV2;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::new(CornellAppV2::default());
    WinitApp::run(outer_app);
}
