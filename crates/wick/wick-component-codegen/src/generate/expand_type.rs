use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use super::dependency::Dependency;
use super::{config, Direction};
use crate::generate::ids::get_typename_parts;

pub(super) fn expand_type(
  config: &mut config::Config,
  dir: Direction,
  imported: bool,
  ty: &wick_interface_types::Type,
) -> TokenStream {
  if config.raw && dir != Direction::Out {
    return quote! { wick_component::packet::Packet };
  }
  match ty {
    wick_interface_types::Type::Bool => quote! { bool },
    wick_interface_types::Type::U8 => quote! { u8 },
    wick_interface_types::Type::U16 => quote! { u16 },
    wick_interface_types::Type::U32 => quote! { u32 },
    wick_interface_types::Type::U64 => quote! { u64 },
    wick_interface_types::Type::I8 => quote! { i8 },
    wick_interface_types::Type::I16 => quote! { i16 },
    wick_interface_types::Type::I32 => quote! { i32 },
    wick_interface_types::Type::I64 => quote! { i64 },
    wick_interface_types::Type::F32 => quote! { f32 },
    wick_interface_types::Type::F64 => quote! { f64 },
    wick_interface_types::Type::String => quote! { String },
    wick_interface_types::Type::List { ty } => {
      let ty = expand_type(config, dir, imported, ty);
      quote! { Vec<#ty> }
    }
    wick_interface_types::Type::Bytes => {
      config.add_dep(Dependency::Bytes);
      quote! {bytes::Bytes}
    }
    wick_interface_types::Type::Custom(name) => {
      let (mod_parts, item_part) = get_typename_parts(name);
      let mod_parts = mod_parts.iter().map(|p| Ident::new(p, Span::call_site()));
      let ty = Ident::new(item_part, Span::call_site());
      let location = if imported {
        quote! {}
      } else {
        quote! {types::}
      };
      quote! {#location #(#mod_parts ::)*#ty}
    }
    wick_interface_types::Type::Optional { ty } => {
      let ty = expand_type(config, dir, imported, ty);
      quote! { Option<#ty> }
    }
    wick_interface_types::Type::Map { key, value } => {
      let key = expand_type(config, dir, imported, key);
      let value = expand_type(config, dir, imported, value);
      quote! { std::collections::HashMap<#key,#value> }
    }
    wick_interface_types::Type::Link { .. } => {
      config.add_dep(Dependency::WickComponent);
      quote! {wick_component::packet::ComponentReference}
    }
    wick_interface_types::Type::Datetime => todo!("implement datetime in new codegen"),
    wick_interface_types::Type::Ref { .. } => todo!("implement ref in new codegen"),
    // wick_interface_types::TypeSignature::Stream { ty } => {
    //   let ty = expand_type(config, dir, imported, ty);
    //   config.add_dep(Dependency::WasmRsRx);
    //   quote! { WickStream<#ty> }
    // }
    wick_interface_types::Type::Object => {
      config.add_dep(Dependency::SerdeJson);
      quote! { Value }
    }
    wick_interface_types::Type::AnonymousStruct(_) => todo!("implement anonymous struct in new codegen"),
  }
}
