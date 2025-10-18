//! enum_variant_accessors
//!
//! Derives:
//! - `EnumIsVariant`: generates `is_<variant_snake>(&self) -> bool`
//! - `EnumAsVariant`: generates `as_<variant_snake>(&self) -> Option<VariantData>`
//!
//! See README.md for details.

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parser, parse_macro_input, parse_quote, Attribute, Data, DataEnum, DeriveInput, Fields,
    GenericParam, Generics, Meta, MetaList, Path, Visibility, WhereClause,
};

/// Whitelist of derives that are safe to mirror onto generated structs.
fn allowed_derives() -> &'static [&'static str] {
    &[
        "Debug",
        "Clone",
        "Copy",
        "PartialEq",
        "Eq",
        "PartialOrd",
        "Ord",
        "Hash",
    ]
}

fn filter_derive_attributes(attrs: &[Attribute]) -> Result<Vec<Attribute>, ()> {
    let mut out = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }

        // parse `derive` inner list: derive(A, B, C)

        let Meta::List(MetaList { tokens, .. }) = &attr.meta else {
            return Err(());
        };

        let parser = syn::punctuated::Punctuated::<Path, syn::Token![,]>::parse_terminated;
        let parsed = parser.parse2(tokens.clone()).unwrap();

        let filtered: Vec<Path> = parsed
            .into_iter()
            .filter(|p| {
                if let Some(ident) = p.get_ident() {
                    allowed_derives().iter().any(|&w| ident == w)
                } else {
                    false
                }
            })
            .collect();

        if !filtered.is_empty() {
            let rebuilt: Attribute = parse_quote!( #[derive( #(#filtered),* )] );
            out.push(rebuilt);
        }
    }

    Ok(out)
}

/// Append a leading lifetime parameter `'a` to generics used for borrowed structs.
fn generics_with_leading_a(orig: &Generics) -> Generics {
    let mut g = orig.clone();
    let lifetime = syn::Lifetime::new("'a", Span::call_site());
    g.params
        .insert(0, GenericParam::Lifetime(syn::LifetimeParam::new(lifetime)));
    g
}

/// The same + leading 'a for borrowed structs.
fn generics_args_with_a(orig: &Generics) -> TokenStream2 {
    let params = std::iter::once(quote! { 'a }).chain(orig.params.iter().map(|p| match p {
        GenericParam::Type(ty) => ty.ident.to_token_stream(),
        GenericParam::Const(c) => c.ident.to_token_stream(),
        GenericParam::Lifetime(lt) => lt.lifetime.to_token_stream(),
    }));
    quote! { < #(#params),* > }
}

fn where_clause_tokens(w: &Option<WhereClause>) -> TokenStream2 {
    match w {
        Some(wc) => wc.to_token_stream(),
        None => quote! {},
    }
}

fn enum_data(input: &DeriveInput) -> Option<&DataEnum> {
    match &input.data {
        Data::Enum(e) => Some(e),
        _ => None,
    }
}

#[proc_macro_derive(EnumIsVariant)]
pub fn derive_enum_is_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Some(data) = enum_data(&input) else {
        return quote! { compile_error!("EnumIsVariant can only be derived for enums."); }.into();
    };

    let enum_ident = &input.ident;
    let generics = input.generics.clone();
    let (impl_generics, _ty_generics, where_clause) = generics.split_for_impl();

    // Generate methods
    let mut methods = Vec::new();
    for v in &data.variants {
        let v_ident = &v.ident;
        let fn_name = Ident::new(
            &format!("is_{}", v_ident.to_string().to_snake_case()),
            v.ident.span(),
        );
        // Pattern (no generics in patterns)
        let pat = match &v.fields {
            Fields::Unit => quote! { #enum_ident::#v_ident },
            Fields::Unnamed(fields) => {
                let blanks = std::iter::repeat(quote! { _ }).take(fields.unnamed.len());
                quote! { #enum_ident::#v_ident( #(#blanks),* ) }
            }
            Fields::Named(_) => quote! { #enum_ident::#v_ident { .. } },
        };
        methods.push(quote! {
            #[inline]
            pub fn #fn_name(&self) -> bool {
                matches!(self, #pat)
            }
        });
    }

    quote! {
        impl #impl_generics #enum_ident #where_clause {
            #(#methods)*
        }
    }
    .into()
}

#[proc_macro_derive(EnumAsVariant)]
pub fn derive_enum_as_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let Some(data) = enum_data(&input) else {
        return quote! { compile_error!("EnumAsVariant can only be derived for enums."); }.into();
    };

    let enum_ident = &input.ident;
    let enum_vis: Visibility = input.vis.clone();
    let generics = input.generics.clone();
    let (impl_generics, _ty_generics, where_clause) = generics.split_for_impl();

    let Ok(filtered_enum_derives) = filter_derive_attributes(&input.attrs) else {
        return quote! { compile_error!("expected the `derive(...)` attribute to contain a list of paths"); }.into();
    };

    let mut helper_structs: Vec<TokenStream2> = Vec::new();
    let mut methods: Vec<TokenStream2> = Vec::new();

    for v in &data.variants {
        let v_ident = &v.ident;
        let fn_name = Ident::new(
            &format!("as_{}", v_ident.to_string().to_snake_case()),
            v.ident.span(),
        );

        match &v.fields {
            Fields::Unit => {
                methods.push(quote! {
                    #[inline]
                    pub fn #fn_name(&self) -> ::core::option::Option<()> {
                        match self {
                            Self::#v_ident => ::core::option::Option::Some(()),
                            _ => ::core::option::Option::None,
                        }
                    }
                });
            }
            Fields::Unnamed(fields) => {
                let n = fields.unnamed.len();
                let refs: Vec<TokenStream2> = fields
                    .unnamed
                    .iter()
                    .map(|f| {
                        let ty = &f.ty;
                        quote! { &'_ #ty }
                    })
                    .collect();

                let ret_ty = if n == 1 {
                    let t0 = &refs[0];
                    quote! { ::core::option::Option<#t0> }
                } else {
                    quote! { ::core::option::Option<( #(#refs),* )> }
                };

                // Bindings with `ref`
                let bindings: Vec<Ident> = (0..n).map(|i| format_ident!("__v{}", i)).collect();
                let pat = {
                    let refs = bindings.iter().map(|b| quote! { ref #b });
                    quote! { Self::#v_ident( #(#refs),* ) }
                };

                let result = if n == 1 {
                    let b0 = &bindings[0];
                    quote! { ::core::option::Option::Some(#b0) }
                } else {
                    quote! { ::core::option::Option::Some( ( #(#bindings),* ) ) }
                };

                methods.push(quote! {
                    #[inline]
                    pub fn #fn_name(&self) -> #ret_ty {
                        match self {
                            #pat => #result,
                            _ => ::core::option::Option::None
                        }
                    }
                });
            }
            Fields::Named(named) => {
                // Helper struct: EnumName + VariantName
                let helper_ident = format_ident!("{}{}", enum_ident, v_ident);

                // Struct generics: 'a + enum generics
                let borrowed_generics = generics_with_leading_a(&generics);
                let borrowed_where = where_clause_tokens(&generics.where_clause);
                let helper_derives = filtered_enum_derives.clone();

                // Fields (public with same attrs), borrowed by &'a Ty
                let field_defs = named.named.iter().map(|f| {
                    let fname = f.ident.as_ref().unwrap();
                    let fattrs = &f.attrs;
                    let fty = &f.ty;
                    quote! {
                        #(#fattrs)*
                        pub #fname: &'a #fty
                    }
                });

                helper_structs.push(quote! {
                    #(#helper_derives)*
                    #enum_vis struct #helper_ident #borrowed_generics #borrowed_where {
                        #(#field_defs),*
                    }
                });

                let ret_ty = {
                    let args = generics_args_with_a(&generics);
                    quote! { ::core::option::Option<#helper_ident #args> }
                };

                let names: Vec<Ident> = named
                    .named
                    .iter()
                    .map(|f| f.ident.clone().unwrap())
                    .collect();
                let pat_fields = names.iter().map(|n| quote! { #n: ref #n });
                let lit_fields = names.iter().map(|n| quote! { #n });

                methods.push(quote! {
                    #[inline]
                    pub fn #fn_name(&self) -> #ret_ty {
                        match self {
                            Self::#v_ident { #(#pat_fields),* } => {
                                ::core::option::Option::Some(
                                    #helper_ident { #(#lit_fields),* }
                                )
                            }
                            _ => ::core::option::Option::None
                        }
                    }
                });
            }
        }
    }

    quote! {
        #(#helper_structs)*

        impl #impl_generics #enum_ident #where_clause {
            #(#methods)*
        }
    }
    .into()
}
