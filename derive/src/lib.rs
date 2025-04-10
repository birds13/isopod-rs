
extern crate proc_macro2;

use proc_macro::TokenStream;
use quote::{format_ident, ToTokens};
use syn::{parse_macro_input, DeriveInput};

fn get_isopod_crate(input: &DeriveInput) -> proc_macro2::TokenStream {
	let mut path = quote::quote! {::isopod};
	if let Some(attr) = input.attrs.iter().find(|attr| attr.path().is_ident("isopod_crate")) {
		let _ = attr.parse_nested_meta(|meta| {
			path = meta.path.to_token_stream();
			Ok(())
		});
	}
	path
}

#[proc_macro_derive(VertexTy, attributes(isopod_crate, position, tex_coord))]
pub fn derive_vertex_ty(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let isopod_crate = get_isopod_crate(&input);
	let ident = input.ident;

	let mut attr_derives = vec![];
	let attributes = match input.data {
		syn::Data::Struct(data_struct) => {
			match data_struct.fields {
				syn::Fields::Named(fields_named) => {
					fields_named.named.iter().map(|field| {
						let name = field.ident.clone();
						let ty = field.ty.clone();
						for attr in field.attrs.iter() {
							if attr.path().is_ident("position") {
								attr_derives.push(quote::quote! {
									impl #isopod_crate::gfx::VertexTyWithPosition for #ident {
										fn set_position(&mut self, v: Vec3) {
											self.#name = v;
										}
										fn get_position(&self) -> Vec3 {
											self.#name
										}
									}
								})
							} else if attr.path().is_ident("tex_coord") {
								attr_derives.push(quote::quote! {
									impl #isopod_crate::gfx::VertexTyWithTexCoord for #ident {
										fn set_tex_coord(&mut self, v: Vec2) {
											self.#name = v;
										}
										fn get_tex_coord(&self) -> Vec2 {
											self.#name
										}
									}
								})
							}
						}
						quote::quote! {
							v.push(#isopod_crate ::gfx::StructAttribute {
								offset: ::std::mem::offset_of!(Self, #name),
								name: stringify!(#name),
								attribute: <#ty as #isopod_crate ::gfx::VertexAttribute>::ID,
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
		impl #isopod_crate ::gfx::VertexTy for #ident {
			fn layout() -> #isopod_crate::gfx::StructLayout<#isopod_crate::gfx::VertexAttributeID> {
				let mut v = Vec::new();
				#(#attributes)*
				#isopod_crate::gfx::StructLayout {
					attributes: v, size: ::std::mem::size_of::<Self>()
				}
			}
		}
		#(#attr_derives)*
	}.into()
}

#[proc_macro_derive(UniformTy, attributes(isopod_crate))]
pub fn derive_uniform_ty(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let isopod_crate = get_isopod_crate(&input);
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
							if end % <#ty as #isopod_crate::gfx::UniformAttribute>::ALIGNMENT != 0 {
								panic!(stringify!(#name is not aligned correctly));
							}
						}
						end
					};
					if <#ty as #isopod_crate::gfx::UniformAttribute>::ID != #isopod_crate::gfx::UniformAttributeID::Padding {
						v.push(#isopod_crate::gfx::StructAttribute {
							offset: #attr_end_name,
							name: stringify!(#name),
							attribute: <#ty as #isopod_crate::gfx::UniformAttribute>::ID,
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
		impl #isopod_crate::gfx::UniformTy for #ident {	
			fn layout() -> #isopod_crate::gfx::StructLayout<#isopod_crate::gfx::UniformAttributeID> {
				let mut v = Vec::new();
				const ATTR_END_BASE: usize = 0;
				#(#attributes)*
				#isopod_crate::gfx::StructLayout { attributes: v, size: ::std::mem::size_of::<Self>() }
			}
		}
	}.into()
}

#[proc_macro_derive(MaterialTy, attributes(isopod_crate))]
pub fn derive_material_ty(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let isopod_crate = get_isopod_crate(&input);
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
							v.push( #isopod_crate::gfx::StructAttribute {
								name: stringify!(#name),
								offset: #offset,
								attribute: <#ty as #isopod_crate::gfx::MaterialAttribute>::id(),
							});
						});
						new_args.push(quote::quote! { #name: &impl #isopod_crate::gfx::MaterialAttributeRef<#ty>, });
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
		impl #isopod_crate::gfx::MaterialTy for #ident {
			fn layout() -> #isopod_crate::gfx::StructLayout<#isopod_crate::gfx::MaterialAttributeID> {
				let mut v = Vec::new();
				#(#layout_pushes)*
				#isopod_crate::gfx::StructLayout { size: v.len(), attributes: v}
			}
		}
		
		impl #ident {
			fn new<'frame>(
				ctx: &'frame #isopod_crate::gfx::GfxCtx,
				#(#new_args)*
			) -> #isopod_crate::gfx::Material<'frame, Self> {
				let mut __v = Vec::new();
				#(#new_pushes)*
				unsafe { #isopod_crate::gfx::Material::from_ref_ids(ctx, __v) }
			}
		}
	}.into()
}