use rust_vk::engine::{Engine, EngineInitInfo};

fn main()
{
    Engine::init(&EngineInitInfo {
        window_width: 800,
        window_height: 800,
        app_name: "hello-triangle".to_string(),
    });

    log::info!("start.");
}
