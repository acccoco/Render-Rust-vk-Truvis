use std::rc::Rc;

use crate::{
    apps::AppInfo,
    platform::{
        application::{Application, ApplicationOptions},
        my_window::MyWindow,
        timer::Timer,
        window::{Window, WindowProperties},
    },
};

/// 这个类用于启动自定义的 application，后面改一下名称
pub struct Platform
{
    window: Option<Rc<dyn Window>>,

    window_properties: WindowProperties,

    active_app: Option<Rc<dyn Application>>,

    requested_app: AppInfo,

    close_requested: bool,

    timer: Timer,
}

impl Platform
{
    pub fn new(app_info: AppInfo) -> Self
    {
        let mut window_properties = WindowProperties::default();
        window_properties.title = app_info.id.clone();
        Self {
            window: None,
            window_properties,
            active_app: None,
            requested_app: app_info,
            close_requested: false,
            timer: Timer::new(),
        }
    }

    pub fn main_loop(&mut self)
    {
        loop {
            if self.window.as_ref().unwrap().should_close() || self.close_requested {
                break;
            }

            if self.active_app.is_none() {
                self.start_app();
            }

            self.update();

            if let Some(active_app) = self.active_app.as_ref() {
                if active_app.should_close() {
                    self.on_app_close();
                    active_app.finish();
                }
            }

            self.window.as_ref().unwrap().process_events();
        }
    }

    pub fn initialize(&mut self) -> anyhow::Result<()>
    {
        simple_logger::SimpleLogger::new().init()?;

        self.window = Some(Rc::new(MyWindow::new(self.window_properties.clone())?));

        Ok(())
    }

    pub fn start_app(&mut self)
    {
        let mut active_app = (self.requested_app.create)();
        active_app.prepare(&ApplicationOptions {
            window: self.window.as_ref().unwrap().clone(),
        });

        self.active_app = Some(Rc::from(active_app));
    }

    pub fn update(&self)
    {
        todo!()
    }

    pub fn on_app_close(&self)
    {
        todo!()
    }
}
