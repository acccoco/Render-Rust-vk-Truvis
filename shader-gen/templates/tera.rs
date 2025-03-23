// 自动生成的代码 - 请勿手动修改

use crate::prelude::*;
use shader_layout_macro::ShaderLayout;

#[derive(ShaderLayout)]
pub struct {{ name }} {
    {% for binding in bindings %}
    #[binding = {{ binding.binding }}]
    pub {{ binding.name }}: {{ binding.rust_type | default(value=binding.type_) }},
    {% endfor %}
}