// //! For interface vs type in typescript we follow https://stackoverflow.com/a/65948871
// //!
// //! ### When to use type:
// //! - Use type when defining an alias for primitive types (string, boolean, number, bigint, symbol, etc)
// //! - Use type when defining tuple types
// //! - Use type when defining function types
// //! - Use type when defining a union
// //! - Use type when trying to overload functions in object types via composition
// //! - Use type when needing to take advantage of mapped types
// //!
// //! ### When to use interface:
// //! - Use interface for all object types where using type is not required (see above)
// //!- Use interface when you want to take advantage of declaration merging.

// use proc_macro2::{Span, TokenStream};
// use quote::quote;
// use syn::parse::Parse;

// trait Ts {
//     fn ty() -> Type;
//     fn with_generics()
//     // fn concrete_type() -> String;
// }

// enum Type {
//     Struct(Struct),
//     // StructUnnamed(StructUnnamed),
//     // NewTypeStruct(NewTypeStruct),
//     // Enum(Enum),
// }

// impl Type {
//     // fn type_definition(&self) -> String {
//     //     match self {
//     //         Type::StructNamed(s) => s.type_definition(),
//     //     }
//     // }

//     // fn concrete_type_with_named_generics(&self, generics: &[Generic]) -> String {
//     //     match self {
//     //         Type::StructNamed(s) => s.concrete_type_with_named_generics(generics),
//     //     }
//     // }
// }

// struct Struct {
//     name: &'static str,
//     generics: Vec<Generic>,
//     fields: StructField,
// }

// enum StructField {
//     Named(Vec<StructFieldNamed>),
//     Unnamed(Vec<StructFieldUnnamed>),
// }

// struct StructFieldNamed {
//     name: String,
//     ty: Box<dyn Fn() -> Type>,
// }

// // impl StructNamed {
// //     fn type_definition(&self) -> String {
// //         self.generics.len();

// //         let mut field_definitions = Vec::new();
// //         for (i, field) in self.fields.iter().enumerate() {
// //             let generics = self.generics_for_fields[i]
// //                 .iter()
// //                 .map(|i| self.generics[*i].clone())
// //                 .collect::<Vec<_>>();
// //             let ty = (field.ty)().concrete_type_with_named_generics(&generics);
// //             field_definitions.push(format!("  {}: {}", field.name, ty));
// //         }

// //         let field_definitions = field_definitions.join(",\n");

// //         if self.generics.is_empty() {
// //             format!("interface {} {{ {} }}", self.name, field_definitions)
// //         } else {
// //             let generic_names = self
// //                 .generics
// //                 .iter()
// //                 .map(|g| g.0.as_str())
// //                 .collect::<Vec<_>>()
// //                 .join(", ");
// //             format!(
// //                 "export interface {}<{}> {{ {} }}",
// //                 self.name, generic_names, field_definitions
// //             )
// //         }
// //     }

// //     fn concrete_type_with_named_generics(&self, generics: &[Generic]) -> String {
// //         if self.generics.is_empty() {
// //             self.name.clone()
// //         } else {
// //             let generic_names = generics
// //                 .iter()
// //                 .map(|g| g.0.as_str())
// //                 .collect::<Vec<_>>()
// //                 .join(", ");
// //             format!("{}<{}>", self.name, generic_names)
// //         }
// //     }
// // }

// #[derive(Clone)]
// enum Generic {};

// struct StructField {
//     name: String,
//     ty: String,
// }

// pub fn typescript(input: TokenStream) -> Result<TokenStream, syn::Error> {
//     let item = syn::parse2::<syn::Item>(input)?;

//     match item {
//         syn::Item::Struct(s) => item_struct(s),
//         // syn::Item::Enum(e) => item_enum(e),
//         _ => Err(syn::Error::new_spanned(
//             item,
//             "Only structs and enums are supported",
//         )),
//     }
// }

// fn item_struct(s: syn::ItemStruct) -> Result<TokenStream, syn::Error> {
//     let name = &s.ident;
//     let generics = &s.generics;
//     let fields = match s.fields {
//         syn::Fields::Named(fields) => fields.named,
//         syn::Fields::Unnamed(fields) => fields.unnamed,
//         syn::Fields::Unit => {
//             return Ok(quote! {
//                 export type #name = null;
//             })
//         }
//     };

//     let mut field_names = Vec::new();
//     let mut field_types = Vec::new();
//     for field in fields {
//         let name = field.ident.unwrap();
//         let ty = field.ty;
//         field_names.push(name);
//         field_types.push(ty);
//     }

//     let generic_names = generics
//         .params
//         .iter()
//         .map(|p| match p {
//             syn::GenericParam::Type(t) => t.ident.to_string(),
//             _ => panic!("Only type generics are supported"),
//         })
//         .collect::<Vec<_>>();
//     let field_generics = field_types
//         .iter()
//         .map(|ty| generics_of_type(ty.clone()))
//         .collect::<Vec<_>>();

//     let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
//     Ok(quote! {
//         impl #impl_generics ts_rpc::Ts for #name #ty_generics #where_clause {
//             fn ty() -> ts_rpc::Type {
//                 ts_rpc::Type::StructNamed(ts_rpc::StructNamed {
//                     name: std::any::type_name::<Self>().to_string(),
//                     generics: vec![],
//                     fields: vec![#(ts_rpc::StructField {
//                         name: stringify!(#field_names),
//                         ty: || <#field_types as ts_rpc::Ts>::ty(),
//                     }),*],
//                     // generics_for_fields: vec![#(vec![#(ts_rpc::Generic(stringify!(#field_types).to_string())),*]),*],
//                 })
//             }
//         }
//     })
// }

// fn generics_of_type(ty: syn::Type) -> Option<Vec<Generic>> {
//     match ty {
//         syn::Type::Path(p) => {
//             if let Some(ty) = p.path.segments.last() {
//                 match ty.arguments {
//                     syn::PathArguments::AngleBracketed(a) => {
//                         let mut generics = Vec::new();
//                         for arg in a.args {
//                             match arg {
//                                 syn::GenericArgument::Type(ty) => generics
//                                     .push(Generic(generics_of_type(ty).unwrap()[0].0.clone())),
//                                 _ => panic!("Only type generics are supported"),
//                             }
//                         }
//                         Some(generics)
//                     }
//                     _ => None,
//                 }
//             }
//         }
//         _ => panic!("Only type generics are supported"),
//     }
// }
