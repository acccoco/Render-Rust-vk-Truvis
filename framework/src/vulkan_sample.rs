use crate::platform::application::{Application, ApplicationInner, ApplicationOptions};

pub struct VulkanSample
{
    application_inner: ApplicationInner,

    instance_extensions: Vec<String>,
}


impl Application for VulkanSample
{
    fn get_inner(&self) -> &ApplicationInner
    {
        &self.application_inner
    }


    fn update(&self, delta_time: f32)
    {
        todo!()
    }

    fn finish(&self)
    {
        todo!()
    }

    fn prepare(&mut self, options: &ApplicationOptions) -> bool
    {
        if !self.application_inner.prepare(options) {
            return false;
        }

        for extension in self.application_inner.window.get_required_surface_extensions() {
            self.add_instance_extension(extension)
        }

        todo!()
    }
}

impl VulkanSample
{
    fn add_instance_extension(&mut self, extension: String)
    {
        self.instance_extensions.push(extension);
    }
}
