use shader_layout_macro::ShaderLayout;

#[derive(ShaderLayout)]
struct MyShader {
    #[binding = 0]
    uniform_buffer: Buffer,

    #[binding = 1]
    texture: Texture,

    #[binding = 2]
    sampler: Sampler,
}

// 模拟类型
struct Buffer;
struct Texture;
struct Sampler;

fn main() {
    // 获取所有绑定信息
    let bindings = MyShader::get_shader_bindings();

    // 输出: [("uniform_buffer", 0), ("texture", 1), ("sampler", 2)]
    println!("Shader bindings: {:?}", bindings);
}
