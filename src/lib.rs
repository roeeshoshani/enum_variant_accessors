//! enum_variant_accessors: derive accessors for enum variants.
//!
//! See README for full documentation and examples.

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, Ident};

#[proc_macro_derive(EnumIsVariant)]
pub fn derive_enum_is_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_is_variant(&input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(EnumAsVariant)]
pub fn derive_enum_as_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_as_variant(&input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_is_variant(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`EnumIsVariant` can only be derived for enums",
        ));
    };

    let mut methods = Vec::new();
    for variant in &data_enum.variants {
        // Reject named struct-like variants
        if matches!(variant.fields, Fields::Named(_)) {
            return Err(syn::Error::new(
                variant.span(),
                "Named-field variants are not supported by `EnumIsVariant`",
            ));
        }

        let v_ident = &variant.ident;
        let snake = v_ident.to_string().to_snake_case();
        let fn_ident = format_ident!("is_{}", snake);

        let pat = match &variant.fields {
            Fields::Unit => quote! { #name::#v_ident },
            Fields::Unnamed(fields) if fields.unnamed.is_empty() => quote! { #name::#v_ident() },
            Fields::Unnamed(fields) => {
                let wilds = std::iter::repeat(quote! { _ })
                    .take(fields.unnamed.len())
                    .collect::<Vec<_>>();
                quote! { #name::#v_ident( #(#wilds),* ) }
            }
            Fields::Named(_) => unreachable!(),
        };

        methods.push(quote! {
            #[inline]
            pub fn #fn_ident(&self) -> bool {
                matches!(self, #pat)
            }
        });
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            #(#methods)*
        }
    };
    Ok(expanded.into())
}

fn impl_as_variant(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`EnumAsVariant` can only be derived for enums",
        ));
    };

    let mut methods = Vec::new();
    for variant in &data_enum.variants {
        // Reject named struct-like variants
        if matches!(variant.fields, Fields::Named(_)) {
            return Err(syn::Error::new(
                variant.span(),
                "Named-field variants are not supported by `EnumAsVariant`",
            ));
        }

        let v_ident = &variant.ident;
        let snake = v_ident.to_string().to_snake_case();

        let as_fn_ident = format_ident!("as_{}", snake);
        let as_mut_fn_ident = format_ident!("as_{}_mut", snake);

        // Build patterns and return types for &self and &mut self accessors
        match &variant.fields {
            Fields::Unit => {
                // Variant -> ()
                methods.push(quote! {
                    #[inline]
                    pub fn #as_fn_ident(&self) -> ::core::option::Option<()> {
                        match self {
                            Self::#v_ident => ::core::option::Option::Some(()),
                            _ => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    pub fn #as_mut_fn_ident(&mut self) -> ::core::option::Option<()> {
                        match self {
                            Self::#v_ident => ::core::option::Option::Some(()),
                            _ => ::core::option::Option::None,
                        }
                    }
                });
            }
            Fields::Unnamed(fields) if fields.unnamed.is_empty() => {
                // Variant() (explicit empty tuple) -> ()
                methods.push(quote! {
                    #[inline]
                    pub fn #as_fn_ident(&self) -> ::core::option::Option<()> {
                        match self {
                            Self::#v_ident() => ::core::option::Option::Some(()),
                            _ => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    pub fn #as_mut_fn_ident(&mut self) -> ::core::option::Option<()> {
                        match self {
                            Self::#v_ident() => ::core::option::Option::Some(()),
                            _ => ::core::option::Option::None,
                        }
                    }
                });
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                // Single-field tuple variant -> &T / &mut T
                let ty = &fields.unnamed[0].ty;
                methods.push(quote! {
                    #[inline]
                    pub fn #as_fn_ident(&self) -> ::core::option::Option<&#ty> {
                        match self {
                            Self::#v_ident(ref v) => ::core::option::Option::Some(v),
                            _ => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    pub fn #as_mut_fn_ident(&mut self) -> ::core::option::Option<&mut #ty> {
                        match self {
                            Self::#v_ident(ref mut v) => ::core::option::Option::Some(v),
                            _ => ::core::option::Option::None,
                        }
                    }
                });
            }
            Fields::Unnamed(fields) => {
                // Multi-field tuple variant -> (&T1, &T2, ...) / (&mut T1, &mut T2, ...)
                let tys: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();

                let idents: Vec<Ident> = (0..tys.len()).map(|i| format_ident!("f{}", i)).collect();

                let ref_pats = idents.iter().map(|id| quote! { ref #id });
                let ref_mut_pats = idents.iter().map(|id| quote! { ref mut #id });

                let tuple_ref = quote! { ( #( & #tys ),* ) };
                let tuple_ref_mut = quote! { ( #( &mut #tys ),* ) };

                let tuple_vals = quote! { ( #( #idents ),* ) };

                methods.push(quote! {
                    #[inline]
                    pub fn #as_fn_ident(&self) -> ::core::option::Option<#tuple_ref> {
                        match self {
                            Self::#v_ident( #( #ref_pats ),* ) => ::core::option::Option::Some(#tuple_vals),
                            _ => ::core::option::Option::None,
                        }
                    }
                    #[inline]
                    pub fn #as_mut_fn_ident(&mut self) -> ::core::option::Option<#tuple_ref_mut> {
                        match self {
                            Self::#v_ident( #( #ref_mut_pats ),* ) => ::core::option::Option::Some(#tuple_vals),
                            _ => ::core::option::Option::None,
                        }
                    }
                });
            }
            Fields::Named(_) => unreachable!(),
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            #(#methods)*
        }
    };
    Ok(expanded.into())
}
