use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Meta};


/// 为结构体实现 ShaderLayout 派生宏
///
/// 支持的属性：
/// - binding: 指定绑定点编号
/// - descriptor_type: 指定描述符类型（如 UNIFORM_BUFFER, COMBINED_IMAGE_SAMPLER 等）
/// - count: 指定描述符数量
/// - stage: 指定着色器阶段（如 VERTEX, FRAGMENT 等）
#[proc_macro_derive(ShaderLayout, attributes(binding, descriptor_type, count, stage))]
pub fn derive_shader_layout(input: TokenStream) -> TokenStream
{
    // 解析输入为 DeriveInput 结构
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // 只处理结构体类型，且只支持具名字段
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    // 收集字段信息：名称、绑定、描述符类型、数量和着色器阶段
    let mut field_infos = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let binding = get_binding_value(&field.attrs);
        let descriptor_type = get_descriptor_type(&field.attrs);
        let count = get_count_value(&field.attrs);
        let stage = get_stage_value(&field.attrs);

        if let Some(binding) = binding {
            field_infos.push((field_name, binding, descriptor_type, count, stage));
        }
    }

    // 生成获取绑定信息的方法
    let field_names = field_infos.iter().map(|(name, ..)| name).collect::<Vec<_>>();
    let binding_values = field_infos.iter().map(|(_, binding, ..)| binding).collect::<Vec<_>>();
    let descriptor_types = field_infos.iter().map(|(_, _, descriptor_type, ..)| descriptor_type).collect::<Vec<_>>();
    let counts = field_infos.iter().map(|(_, _, _, count, _)| count).collect::<Vec<_>>();
    let stages = field_infos.iter().map(|(_, _, _, _, stage)| stage).collect::<Vec<_>>();

    // 生成代码：
    // 1. 实现 get_shader_bindings 方法，返回字段名和绑定值的元组数组
    // 2. 实现 ShaderBindingLayout trait，返回完整的 ShaderBindingItem 数组
    let expanded = quote! {
        impl shader_layout_trait::ShaderBindingLayout for #struct_name {
            fn get_shader_bindings() -> Vec<shader_layout_trait::ShaderBindingItem> {
                vec![
                    #(shader_layout_trait::ShaderBindingItem {
                        name: stringify!(#field_names),
                        binding: #binding_values,
                        descriptor_type: #descriptor_types,
                        stage_flags: #stages,
                        count: #counts,
                    }),*
                ]
            }
        }
    };

    expanded.into()
}

/// 从字段属性中获取 binding 值
///
/// 属性格式示例：#[binding = 0]
fn get_binding_value(attrs: &[Attribute]) -> Option<u32>
{
    for attr in attrs {
        if attr.path().is_ident("binding") {
            if let Meta::NameValue(meta) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(lit_int),
                    ..
                }) = &meta.value
                {
                    return Some(lit_int.base10_parse().unwrap());
                }
            }
        }
    }
    None
}

/// 从字段属性中获取 descriptor_type 值
///
/// 属性格式示例：#[descriptor_type = "UNIFORM_BUFFER"]
fn get_descriptor_type(attrs: &[Attribute]) -> syn::Expr
{
    for attr in attrs {
        if attr.path().is_ident("descriptor_type") {
            if let Meta::NameValue(meta) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &meta.value
                {
                    let descriptor_type = format!("vk::DescriptorType::{}", lit_str.value().as_str());
                    return syn::parse_str(&descriptor_type).unwrap();
                }
            }
        }
    }
    // 默认值：统一缓冲区
    syn::parse_quote!(vk::DescriptorType::UNIFORM_BUFFER)
}

/// 从字段属性中获取 count 值
///
/// 属性格式示例：#[count = 1]
fn get_count_value(attrs: &[Attribute]) -> syn::Expr
{
    for attr in attrs {
        if attr.path().is_ident("count") {
            if let Meta::NameValue(meta) = &attr.meta {
                return meta.value.clone();
            }
        }
    }
    // 默认值：1
    syn::parse_quote!(1)
}

/// 从字段属性中获取 stage 值
///
/// 属性格式示例：#[stage = "VERTEX | FRAGMENT"]
/// 支持的着色器阶段：
/// - VERTEX
/// - FRAGMENT
///
/// 多个阶段可以用 | 连接
fn get_stage_value(attrs: &[Attribute]) -> syn::Expr
{
    for attr in attrs {
        if attr.path().is_ident("stage") {
            if let Meta::NameValue(meta) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &meta.value
                {
                    let stage = lit_str.value();
                    let stage_flags = stage
                        .split(" | ")
                        .map(|s| format!("vk::ShaderStageFlags::{}", s))
                        .collect::<Vec<_>>()
                        .join(" | ");
                    return syn::parse_str(&stage_flags).unwrap();
                }
            }
        }
    }
    // 默认值：顶点和片段着色器
    syn::parse_quote!(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
}
