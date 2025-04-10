
extern crate proc_macro2;

use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
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

fn derive_data_struct(input: &DeriveInput, mut process_fn: impl FnMut(&syn::Field)) -> proc_macro2::TokenStream {
	let ident = input.ident.clone();
	if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
		panic!("generics are not supported for this trait");
	}
	let mut sizes = vec![];
	match &input.data {
		syn::Data::Struct(data_struct) => {
			match &data_struct.fields {
				syn::Fields::Named(fields_named) => {
					if fields_named.named.is_empty() {
						panic!("struct must not be empty");
					}
					for field in fields_named.named.iter() {
						process_fn(field);
						for attr in field.attrs.iter() {
							if attr.path().is_ident("repr") {
								attr.parse_nested_meta(|meta| {
									if !meta.path.is_ident("C") {
										panic!("struct must be repr(C)");
									}
									Ok(())
								}).unwrap();
							}
						}
						let ty = field.ty.clone();
						sizes.push(quote::quote! { + ::std::mem::size_of::<#ty>() });
					}
				},
 				syn::Fields::Unnamed(_) => panic!("must have named fields"),
    			syn::Fields::Unit => panic!("cannot be a unit type"),
			}
		},
		_ => panic!("type must be a struct")
	};
	quote! {
		const __SIZE: usize = {
			let attr_sum = 0 #(#sizes)* ;
			let struct_size = ::std::mem::size_of::<#ident>();
			if struct_size != attr_sum {
				panic!("struct contains implicit padding - either make this padding explicit using isopod::gfx::Padding or rearrange fields to remove it");
			}
			struct_size
		};
	}
}

#[proc_macro_derive(VertexTy, attributes(isopod_crate, position, tex_coord))]
pub fn derive_vertex_ty(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let isopod_crate = get_isopod_crate(&input);
	let ident = input.ident.clone();

	let mut attributes = vec![];
	let mut attr_derives = vec![];
	let size = derive_data_struct(&input, |field| {
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
		attributes.push(quote::quote! {
			v.push(#isopod_crate ::gfx::StructAttribute {
				offset: ::std::mem::offset_of!(Self, #name),
				name: stringify!(#name),
				attribute: <#ty as #isopod_crate ::gfx::VertexAttribute>::ID,
			});
		});
	});
	quote::quote! {
		unsafe impl #isopod_crate ::gfx::VertexTy for #ident {
			fn layout() -> #isopod_crate::gfx::StructLayout<#isopod_crate::gfx::VertexAttributeID> {
				#size
				let mut v = Vec::new();
				#(#attributes)*
				#isopod_crate::gfx::StructLayout {
					attributes: v, size: __SIZE
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
	let ident = input.ident.clone();
	
	let mut prev_attr_end_name = format_ident!("ATTR_END_BASE");
	let mut attributes = vec![];
	let mut i: usize = 0;
	let size = derive_data_struct(&input, |field| {
		let ty = field.ty.clone();
		let name = field.ident.clone();
		let attr_end_name = format_ident!("ATTR_END_{}", i);
		attributes.push(quote::quote! {
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
		});
		prev_attr_end_name = attr_end_name;
		i += 1;
	});
	quote::quote! {
		unsafe impl #isopod_crate::gfx::UniformTy for #ident {	
			fn layout() -> #isopod_crate::gfx::StructLayout<#isopod_crate::gfx::UniformAttributeID> {
				#size
				let mut v = Vec::new();
				const ATTR_END_BASE: usize = 0;
				#(#attributes)*
				#isopod_crate::gfx::StructLayout { attributes: v, size: __SIZE }
			}
		}
	}.into()
}