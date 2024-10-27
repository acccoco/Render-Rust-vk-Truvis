use std::rc::Rc;

use crate::platform::window::Window;

pub struct ApplicationInner
{
    fps: f32,
    frame_time: f32,
    frame_count: u32,
    last_frame_count: u32,
    lock_simulation_speed: bool,
    pub window: Rc<dyn Window>,

    name: String,
    requested_close: bool,
}

pub trait Application
{
    fn get_inner(&self) -> &ApplicationInner;

    fn update(&self, delta_time: f32);

    fn should_close(&self) -> bool
    {
        self.get_inner().requested_close
    }

    fn finish(&self);

    fn prepare(&mut self, options: &ApplicationOptions) -> bool;
}

impl Application for ApplicationInner
{
    fn get_inner(&self) -> &ApplicationInner
    {
        &self
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
        self.window = options.window.clone();
        true
    }
}


pub struct ApplicationOptions
{
    pub window: Rc<dyn Window>,
}
