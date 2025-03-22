use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Meta};

#[proc_macro_derive(ShaderLayout, attributes(binding))]
pub fn derive_shader_layout(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // 只处理结构体
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    // 收集字段信息
    let mut field_infos = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let binding = get_binding_value(&field.attrs);

        if let Some(binding) = binding {
            field_infos.push((field_name, binding));
        }
    }

    // 生成获取绑定信息的方法
    let field_names = field_infos.iter().map(|(name, _)| name);
    let binding_values = field_infos.iter().map(|(_, binding)| binding);

    let expanded = quote! {
        impl #struct_name {
            pub fn get_shader_bindings() -> Vec<(&'static str, u32)> {
                vec![
                    #((stringify!(#field_names), #binding_values)),*
                ]
            }
        }
    };

    expanded.into()
}

// 辅助函数：从属性中获取 binding 值
fn get_binding_value(attrs: &[Attribute]) -> Option<u32> {
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
