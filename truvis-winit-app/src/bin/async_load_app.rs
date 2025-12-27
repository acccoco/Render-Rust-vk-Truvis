use truvis_app::outer_app::async_load_test::async_load_app::AsyncLoadTest;
use truvis_winit_app::app::WinitApp;

fn main() {
    let outer_app = Box::new(AsyncLoadTest::default());
    WinitApp::run(outer_app);
}
