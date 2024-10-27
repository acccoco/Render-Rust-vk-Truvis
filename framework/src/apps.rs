use crate::platform::application::Application;

pub struct AppInfo
{
    pub id: String,
    pub create: Box<dyn Fn() -> Box<dyn Application>>,
}
