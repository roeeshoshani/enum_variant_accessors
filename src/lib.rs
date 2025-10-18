//! # enum_variant_accessors
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
    parse_macro_input, Attribute, Data, DataEnum, DeriveInput, Fields, GenericParam, Generics,
    Meta, MetaList, NestedMeta, Type, Visibility, WhereClause,
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

fn filter_derive_attributes(attrs: &[Attribute]) -> Vec<Attribute> {
    let mut out = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("derive") {
            continue;
        }
        let Ok(Meta::List(MetaList { tokens, .. })) = attr.parse_meta() else {
            continue;
        };
        // Parse items inside #[derive(...)]
        let nested = syn::parse2::<syn::punctuated::Punctuated<syn::Path, syn::Token![,]>>(tokens)
            .unwrap_or_default();

        let filtered: Vec<syn::Path> = nested
            .into_iter()
            .filter(|p| {
                if let Some(ident) = p.get_ident() {
                    allowed_derives().iter().any(|allowed| ident == allowed)
                } else {
                    false
                }
            })
            .collect();

        if !filtered.is_empty() {
            let rebuilt = quote! { #[derive( #(#filtered),* )] };
            out.push(syn::parse2::<Attribute>(rebuilt).expect("rebuild derive attr"));
        }
    }
    out
}

/// Append a leading lifetime parameter `'a` to generics used for borrowed structs.
fn generics_with_leading_a(orig: &Generics) -> Generics {
    let mut g = orig.clone();
    let lifetime = syn::Lifetime::new("'a", Span::call_site());
    g.params
        .insert(0, GenericParam::Lifetime(syn::LifetimeParam::new(lifetime)));
    g
}

/// The same generics but used in type paths (angle bracket args).
fn generics_args(orig: &Generics) -> TokenStream2 {
    let params = orig.params.iter().map(|p| match p {
        GenericParam::Type(ty) => ty.ident.to_token_stream(),
        GenericParam::Const(c) => c.ident.to_token_stream(),
        GenericParam::Lifetime(lt) => lt.lifetime.to_token_stream(),
    });
    if orig.params.is_empty() {
        quote! {}
    } else {
        quote! { < #(#params),* > }
    }
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

fn enum_data(input: &DeriveInput) -> &DataEnum {
    match &input.data {
        Data::Enum(e) => e,
        _ => abort(input.ident.span(), "Derive only supported for enums."),
    }
}

fn abort(span: Span, msg: &str) -> ! {
    proc_macro_error::emit_error!(span, "{}", msg);
    panic!("{}", msg);
}

#[proc_macro_derive(EnumIsVariant)]
pub fn derive_enum_is_variant(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_ident = &input.ident;
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let data = enum_data(&input);

    let is_methods = data.variants.iter().map(|v| {
        let v_ident = &v.ident;
        let fn_name = Ident::new(
            &format!("is_{}", v_ident.to_string().to_snake_case()),
            v.ident.span(),
        );
        // match any shape of fields
        let pat = match &v.fields {
            Fields::Unit => quote! { #enum_ident::#v_ident },
            Fields::Unnamed(fields) => {
                let blanks = std::iter::repeat(quote! { _ }).take(fields.unnamed.len());
                quote! { #enum_ident::#v_ident( #(#blanks),* ) }
            }
            Fields::Named(_) => quote! { #enum_ident::#v_ident { .. } },
        };
        quote! {
            #[inline]
            pub fn #fn_name(&self) -> bool {
                matches!(self, #pat #ty_generics)
            }
        }
    });

    let expanded = quote! {
        impl #impl_generics #enum_ident #ty_generics #where_clause {
            #(#is_methods)*
        }
    };

    expanded.into()
}

#[proc_macro_derive(EnumAsVariant)]
pub fn derive_enum_as_variant(input: TokenStream) -> TokenStream {
    // We need to potentially generate helper structs for named variants
    let input = parse_macro_input!(input as DeriveInput);
    let enum_ident = &input.ident;
    let enum_vis: Visibility = input.vis.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let data = enum_data(&input);

    let filtered_enum_derives = filter_derive_attributes(&input.attrs);

    // For each named-field variant, generate a borrowed struct:
    let mut helper_structs: Vec<TokenStream2> = Vec::new();

    // Methods `as_*`:
    let as_methods = data.variants.iter().map(|v| {
        let v_ident = &v.ident;
        let snake = v_ident.to_string().to_snake_case();
        let fn_name = Ident::new(&format!("as_{}", snake), v.ident.span());

        match &v.fields {
            Fields::Unit => {
                // Option<()>
                quote! {
                    #[inline]
                    pub fn #fn_name(&self) -> ::core::option::Option<()> {
                        match self {
                            Self::#v_ident => ::core::option::Option::Some(()),
                            _ => ::core::option::Option::None
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                // Single → &T
                // Multiple → (&T1, &T2, ...)
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

                // pattern with refs: Variant(ref a, ref b, ...)
                let bindings: Vec<Ident> = (0..n).map(|i| format_ident!("__v{}", i)).collect();
                let pat = {
                    let refs = bindings.iter().map(|b| quote! { ref #b });
                    quote! { Self::#v_ident( #(#refs),* ) }
                };

                let result = if n == 1 {
                    let b0 = &bindings[0];
                    quote! { ::core::option::Option::Some(#b0) }
                } else {
                    quote! { ::core::option::Option::Some( ( #(&#bindings),* ) ) }
                };

                quote! {
                    #[inline]
                    pub fn #fn_name(&self) -> #ret_ty {
                        match self {
                            #pat => #result,
                            _ => ::core::option::Option::None
                        }
                    }
                }
            }
            Fields::Named(named) => {
                // Build/remember helper struct: EnumName + VariantName
                let helper_ident = format_ident!("{}{}", enum_ident, v_ident);
                // Struct generics: 'a + enum generics
                let borrowed_generics = generics_with_leading_a(&generics);
                let borrowed_where = where_clause_tokens(&generics.where_clause);
                let helper_derives = filtered_enum_derives.clone();

                // Fields: &'a Ty, with original field attrs copied verbatim
                let field_defs = named.named.iter().map(|f| {
                    let fname = f.ident.as_ref().unwrap();
                    let fattrs = &f.attrs;
                    let fty = &f.ty;
                    quote! {
                        #(#fattrs)*
                        pub #fname: &'a #fty
                    }
                });

                // Visibility mirrors enum's visibility
                helper_structs.push(quote! {
                    #(#helper_derives)*
                    #enum_vis struct #helper_ident #borrowed_generics #borrowed_where {
                        #(#field_defs),*
                    }
                });

                // as_method returns Option<Helper<'_ , ...>>
                let ret_ty = {
                    let args = generics_args_with_a(&generics);
                    quote! { ::core::option::Option<#helper_ident #args> }
                };

                // Build bindings and struct literal
                let names: Vec<Ident> = named
                    .named
                    .iter()
                    .map(|f| f.ident.clone().unwrap())
                    .collect();
                let pat_fields = names.iter().map(|n| quote! { #n: ref #n });
                let lit_fields = names.iter().map(|n| quote! { #n });

                quote! {
                    #[inline]
                    pub fn #fn_name(&self) -> #ret_ty {
                        match self {
                            Self::#v_ident { #(#pat_fields),* } => {
                                ::core::option::Option::Some(
                                    #helper_ident {
                                        #(#lit_fields),*
                                    }
                                )
                            }
                            _ => ::core::option::Option::None
                        }
                    }
                }
            }
        }
    });

    let helpers_block = if helper_structs.is_empty() {
        quote! {}
    } else {
        quote! { #(#helper_structs)* }
    };

    let expanded = quote! {
        #helpers_block

        impl #impl_generics #enum_ident #ty_generics #where_clause {
            #(#as_methods)*
        }
    };
    expanded.into()
}
