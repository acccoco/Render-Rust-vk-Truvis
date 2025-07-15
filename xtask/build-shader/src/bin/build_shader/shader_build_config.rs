/// 当前项目的环境路径，基于 workspace 根目录
pub struct EnvPath;
impl EnvPath {
    /// shader 所在的路径
    pub const SHADER_DIR: &'static str = "shader";

    /// 编译 shader 的输出路径
    pub const SHADER_BUILD_DIR: &'static str = "shader/build";

    /// shader 的 include 目录
    pub const SHADER_INCLUDE_DIR: &'static str = "shader/include";

    /// slang shader 编译器的路径
    pub const SLANGC_PATH: &'static str = "tools/slang/bin/slangc.exe";
}
