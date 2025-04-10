
extern crate proc_macro2;

use proc_macro::TokenStream;
use quote::format_ident;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(VertexTy)]
pub fn derive_vertex_ty(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = input.ident;

	let attributes = match input.data {
		syn::Data::Struct(data_struct) => {
			match data_struct.fields {
				syn::Fields::Named(fields_named) => {
					fields_named.named.iter().map(|field| {
						let name = field.ident.clone();
						let ty = field.ty.clone();
						quote::quote! {
							v.push(::isopod::gfx::StructAttribute {
								offset: ::std::mem::offset_of!(Self, #name),
								name: stringify!(#name),
								attribute: <#ty as ::isopod::gfx::VertexAttribute>::ID,
							});
						}
					}).collect::<Vec<_>>()
				},
 				syn::Fields::Unnamed(_) => panic!("must have named fields"),
    			syn::Fields::Unit => panic!("cannot be a unit type"),
			}
		},
		_ => panic!("type must be a struct")
	};
	quote::quote! {
		impl ::isopod::gfx::VertexTy for #ident {
			fn layout() -> ::isopod::gfx::StructLayout<::isopod::gfx::VertexAttributeID> {
				let mut v = Vec::new();
				#(#attributes)*
				::isopod::gfx::StructLayout {
					attributes: v, size: ::std::mem::size_of::<Self>()
				}
			}
		}
	}.into()
}

#[proc_macro_derive(UniformTy)]
pub fn derive_uniform_ty(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = input.ident;
	
	let mut prev_attr_end_name = format_ident!("ATTR_END_BASE");
	let attributes = match input.data {
		syn::Data::Struct(data_struct) => match data_struct.fields {
			syn::Fields::Named(fields_named) => fields_named.named.iter().enumerate().map(|(i, field)| {
				let ty = field.ty.clone();
				let name = field.ident.clone();
				let attr_end_name = format_ident!("ATTR_END_{}", i);
				let code = quote::quote! {
					const #attr_end_name: usize = {
						let end = ::std::mem::offset_of!(#ident, #name);
						if #prev_attr_end_name != 0 {
							if end % <#ty as ::isopod::gfx::UniformAttribute>::ALIGNMENT != 0 {
								panic!(stringify!(#name is not aligned correctly));
							}
						}
						end
					};
					if <#ty as ::isopod::gfx::UniformAttribute>::ID != ::isopod::gfx::UniformAttributeID::Padding {
						v.push(::isopod::gfx::StructAttribute {
							offset: #attr_end_name,
							name: stringify!(#name),
							attribute: <#ty as ::isopod::gfx::UniformAttribute>::ID,
						});
					}
				};
				prev_attr_end_name = attr_end_name;
				code
			}).collect::<Vec<_>>(),
			syn::Fields::Unnamed(_) => panic!("must have named fields"),
			syn::Fields::Unit => panic!("cannot be a unit type"),
		},
		_ => panic!("type must be a struct")
	};
	quote::quote! {
		impl ::isopod::gfx::UniformTy for #ident {	
			fn layout() -> ::isopod::gfx::StructLayout<::isopod::gfx::UniformAttributeID> {
				let mut v = Vec::new();
				const ATTR_END_BASE: usize = 0;
				#(#attributes)*
				::isopod::gfx::StructLayout { attributes: v, size: ::std::mem::size_of::<Self>() }
			}
		}
	}.into()
}

#[proc_macro_derive(MaterialTy)]
pub fn derive_material_ty(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = input.ident;
	
	let mut layout_pushes = vec![];
	let mut new_args = vec![];
	let mut new_pushes = vec![];
	match input.data {
		syn::Data::Struct(data_struct) => {
			match data_struct.fields {
				syn::Fields::Named(fields_named) => {
					for field in fields_named.named.iter() {
						let name = field.ident.clone();
						let ty = field.ty.clone();
						let offset = proc_macro2::Literal::usize_unsuffixed(layout_pushes.len());
						layout_pushes.push(quote::quote! {
							v.push( ::isopod::gfx::StructAttribute {
								name: stringify!(#name),
								offset: #offset,
								attribute: <#ty as ::isopod::gfx::MaterialAttribute>::id(),
							});
						});
						new_args.push(quote::quote! { #name: &impl ::isopod::gfx::MaterialAttributeRef<#ty>, });
						new_pushes.push(quote::quote! {
							__v.push(#name.id());
						});
					}
				},
 				syn::Fields::Unnamed(_) => panic!("must have named fields"),
    			syn::Fields::Unit => panic!("cannot be a unit type"),
			}
		},
		_ => panic!("type must be a struct")
	};
	quote::quote! {
		impl ::isopod::gfx::MaterialTy for #ident {
			fn layout() -> ::isopod::gfx::StructLayout<::isopod::gfx::MaterialAttributeID> {
				let mut v = Vec::new();
				#(#layout_pushes)*
				::isopod::gfx::StructLayout { size: v.len(), attributes: v}
			}
		}
		
		impl #ident {
			fn new<'frame>(
				ctx: &'frame ::isopod::gfx::GfxCtx,
				#(#new_args)*
			) -> ::isopod::gfx::Material<'frame, Self> {
				let mut __v = Vec::new();
				#(#new_pushes)*
				unsafe { ::isopod::gfx::Material::from_ref_ids(ctx, __v) }
			}
		}
	}.into()
}