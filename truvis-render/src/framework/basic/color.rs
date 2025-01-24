pub struct LabelColor;
impl LabelColor
{
    const RED: glam::Vec4 = glam::vec4(1.0, 0.0, 0.0, 1.0);
    const GREEN: glam::Vec4 = glam::vec4(0.0, 1.0, 0.0, 1.0);
    const BLUE: glam::Vec4 = glam::vec4(0.0, 0.0, 1.0, 1.0);
    const WHITE: glam::Vec4 = glam::vec4(1.0, 1.0, 1.0, 1.0);
    const BLACK: glam::Vec4 = glam::vec4(0.0, 0.0, 0.0, 1.0);
    const YELLOW: glam::Vec4 = glam::vec4(1.0, 1.0, 0.0, 1.0);
    const CYAN: glam::Vec4 = glam::vec4(0.0, 1.0, 1.0, 1.0);
    const MAGENTA: glam::Vec4 = glam::vec4(1.0, 0.0, 1.0, 1.0);
    const GRAY: glam::Vec4 = glam::vec4(0.5, 0.5, 0.5, 1.0);
    const LIGHT_GRAY: glam::Vec4 = glam::vec4(0.75, 0.75, 0.75, 1.0);
    const DARK_GRAY: glam::Vec4 = glam::vec4(0.25, 0.25, 0.25, 1.0);


    pub const COLOR_PASS: glam::Vec4 = Self::BLUE;
    pub const COLOR_STAGE: glam::Vec4 = Self::YELLOW;
    pub const COLOR_CMD: glam::Vec4 = Self::GREEN;
}
