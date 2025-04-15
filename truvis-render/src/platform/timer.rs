use chrono::{DateTime, Local};

pub struct Timer
{
    running: bool,
    lapping: bool,

    start_time: DateTime<Local>,
    previous_tiek: DateTime<Local>,
    lap_time: DateTime<Local>,
}

impl Timer
{
    pub fn new() -> Timer
    {
        todo!()
    }

    pub fn start(&self)
    {
        todo!()
    }
}
