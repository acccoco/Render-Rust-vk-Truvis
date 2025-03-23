// 自动生成的代码 - 请勿手动修改

{% for binding in bindings %}
{% if binding.type_ == "uniform_buffer" %}
cbuffer {{ binding.name }} : register(b{{ binding.binding }}) {
    {{ binding.hlsl_type }} {{ binding.name }};
}
{% elif binding.type_ == "texture2d" %}
Texture2D {{ binding.name }} : register(t{{ binding.binding }});
{% elif binding.type_ == "sampler" %}
SamplerState {{ binding.name }} : register(s{{ binding.binding }});
{% endif %}
{% endfor %}