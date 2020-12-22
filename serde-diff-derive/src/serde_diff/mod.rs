extern crate proc_macro;

mod args;

use quote::{quote, format_ident};

/// Reads in all tokens for the struct having the
pub fn macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use darling::FromDeriveInput;
    use syn::Data;
    
    // Parse the struct
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_args = args::SerdeDiffStructArgs::from_derive_input(&input).unwrap();
    let target_type = if let Some(ref target) = struct_args.target {
        let span = proc_macro2::Span::call_site();
        let target_type_result = parse_string_to_type(target.to_owned(), span);
        Some(target_type_result.unwrap()) // is there something useful we could do with this error?
    } else {
        None
    };
    match input.data {
        Data::Struct(..) | Data::Enum(..) => {
            if struct_args.opaque {
                 generate_opaque(&input, struct_args)
             } else {
                 // Go ahead and generate the code                 
                 match generate(&input, struct_args, target_type) {
                     Ok(v) => v,
                     Err(v) => v,
                 }
             }
        }
        _ => unimplemented!()
    }
}

/// Called per field to parse and verify it
fn parse_field(f: &syn::Field) -> Result<ParsedField, darling::Error> {
    use darling::FromField;
    let field_args = args::SerdeDiffFieldArgs::from_field(&f)?;
    Ok(ParsedField { field_args })
}

fn parse_fields(input: &syn::Fields) -> Vec<Result<ParsedField, darling::Error>> {
    use syn::Fields;
    match input {
        Fields::Named(ref fields) => fields.named.iter().map(|f| parse_field(&f)).collect(),
        Fields::Unnamed(ref fields) => fields.unnamed.iter().map(|f| parse_field(&f)).collect(),
        Fields::Unit => vec![],
    }
}

/// Parsed metadata for a field
#[derive(Debug)]
struct ParsedField {
    field_args: args::SerdeDiffFieldArgs,
}

fn generate_fields_diff(
    parsed_fields: &[ParsedField],
    matching : bool,
) -> proc_macro2::TokenStream {
    // This will hold a bit of code per-field that call diff on that field
    let mut diff_fn_field_handlers = vec![];
    for (field_idx, pf) in parsed_fields.iter().enumerate() {
        // Skip fields marked as #[serde_diff(skip)]
        if pf.field_args.skip() {
            continue;
        }

        let ident = pf.field_args.ident().clone();
        let ident_as_str = quote!(#ident).to_string();
        let ty = pf.field_args.ty();
        let field_idx = field_idx as u16;
        let left = format_ident!("l{}", field_idx);
        let right = format_ident!("r{}", field_idx);

        let push = if let Some(_) = ident {
            quote!{ctx.push_field(#ident_as_str);}
        } else {
            quote!{ctx.push_field_index(#field_idx);}
        };

        if pf.field_args.opaque() || !matching {
            let cmp = if matching {
                quote! { #left != #right }
            } else {
                quote! {true}
            };
            diff_fn_field_handlers.push(quote! {
                {
                    #push
                    if #cmp {
                        ctx.save_value(&#right)?;
                        __changed__ |= true;
                    }
                    ctx.pop_path_element()?;
                }
            });
        } else {
            diff_fn_field_handlers.push(quote! {
                {
                    {
                        #push
                        __changed__ |= <#ty as serde_diff::SerdeDiff>::diff(&#left, ctx, &#right)?;
                        ctx.pop_path_element()?;
                    }
                }
            });
        }
    }
    quote! {
        #(#diff_fn_field_handlers)*
    }
}


fn enum_fields(fields : &syn::Fields, mutable: bool) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    use syn::Fields;
    let field_match = |f: &syn::Field, name, idx| {
        let name = format_ident!("{}{}", name, idx);
        let mut_tok = if mutable {
            Some(quote!(ref mut))
        } else {
            None
        };
        if let Some(n) = &f.ident {
            quote! { #n :  #mut_tok #name }
        } else {
            quote! {#mut_tok #name}
        }
    };
    let fields_match = |fields: &syn::Fields, prefix| {
        match fields {
            Fields::Named(n) => {
                n.named.iter().enumerate().map(|(i, f)| field_match(f, prefix, i)).collect()
            },
            Fields::Unnamed(n) => {
                n.unnamed.iter().enumerate().map(|(i, f)| field_match(f, prefix, i)).collect()
            },
            Fields::Unit => vec![]
        }
    };
    let (left, right) = (fields_match(fields, "l"), fields_match(fields, "r"));    
    let (left, right) = match fields {
        Fields::Named(_)  => (quote!{{#(#left),*}}, quote!{{#(#right),*}}),
        Fields::Unnamed(_)  => (quote!{(#(#left),*)}, quote!{(#(#right),*)}),
        Fields::Unit => (quote!{}, quote!{}),
    };
    (left, right)
}

fn ok_fields(fields : &syn::Fields) -> Result<Vec<ParsedField>, proc_macro::TokenStream> {
    let parsed_fields = parse_fields(fields);
    // Check all parsed fields for any errors that may have occurred
    let mut ok_fields: Vec<ParsedField> = vec![];
    let mut errors = vec![];
    for pf in parsed_fields {
        match pf {
            Ok(value) => ok_fields.push(value),
            Err(e) => errors.push(e),
        }
    }
    // If any error occurred, return them all here
    if !errors.is_empty() {
       Err(proc_macro::TokenStream::from(darling::Error::multiple(errors).write_errors()))
    } else {
        Ok(ok_fields)
    }
}

fn generate_arms(name: &syn::Ident, variant: Option<&syn::Ident>, fields: &syn::Fields, matching: bool)
                 -> Result<(Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>),
                           proc_macro2::TokenStream>
{
    let mut diff_match_arms = vec![];
    let mut apply_match_arms = vec![];
    let parsed_fields = ok_fields(&fields)?;
    let diffs = generate_fields_diff(
        &parsed_fields,
        matching,
    );                    
    let (left, right) =  enum_fields(&fields, false);
    let variant_specifier = if let Some(id) = variant {
        quote!{ :: #id}
    } else {
        quote!{}
    };
    
    let variant_as_str = variant.map(|i| i.to_string());
    let push_variant = variant.map(|_| quote!{ctx.push_variant(#variant_as_str);});
    let pop_variant = variant.map(|_| quote!{ctx.pop_path_element()?;});
    
    let left = if matching {
        quote! { #name #variant_specifier #left }
    } else {
        quote! {_}
    };
    if matching {
        diff_match_arms.push(                  
            quote!{
                (#left, #name #variant_specifier #right) => {
                    #push_variant
                    #diffs
                    #pop_variant
                }
            }
        );
    } else {
        diff_match_arms.push(                  
            quote!{
                (#left, #name #variant_specifier #right) => {
                    ctx.push_full_variant();
                    ctx.save_value(other)?;
                    ctx.pop_path_element()?;
                }
            }
        );  
    }
    
    if matching {
        let (left, _right) =  enum_fields(fields, true);
        let mut apply_fn_field_handlers = vec![];
        for (field_idx, pf) in parsed_fields.iter().enumerate() {
            // Skip fields marked as #[serde_diff(skip)]
            if pf.field_args.skip() {
                continue;
            }

            let ident = pf.field_args.ident().clone();
            let ty = pf.field_args.ty();
            let field_idx = field_idx as u16;

            let lhs = format_ident!("l{}", field_idx);
            if pf.field_args.opaque() {
                apply_fn_field_handlers.push(quote!(
                    serde_diff::DiffPathElementValue::FieldIndex(#field_idx) =>
                        __changed__ |= ctx.read_value(seq, #lhs)?,
                ));
                if let Some(ident_as_str) = ident.map(|s| s.to_string()) {
                    apply_fn_field_handlers.push(quote!(
                        serde_diff::DiffPathElementValue::Field(field) if field == #ident_as_str =>
                            __changed__ |= ctx.read_value(seq, #lhs)?,
                    ));
                }
            } else {
                apply_fn_field_handlers.push(quote!(
                    serde_diff::DiffPathElementValue::FieldIndex(#field_idx) =>
                        __changed__ |= <#ty as serde_diff::SerdeDiff>::apply(#lhs, seq, ctx)?,
                ));
                if let Some(ident_as_str) = ident.map(|s| s.to_string()) {
                    apply_fn_field_handlers.push(quote!(
                        serde_diff::DiffPathElementValue::Field(field) if field == #ident_as_str => 
                            __changed__ |= <#ty as serde_diff::SerdeDiff>::apply(#lhs, seq, ctx)?,
                    ));
                }
            }
        }

        if let Some(_) = variant {
            apply_match_arms.push(quote!{
                ( &mut #name #variant_specifier #left, Some(serde_diff::DiffPathElementValue::EnumVariant(variant))) if variant == #variant_as_str => {
                    while let Some(element) = ctx.next_path_element(seq)? {
                        match element {
                            #(#apply_fn_field_handlers)* 
                            _ =>  ctx.skip_value(seq)?
                        }
                    }
                }
            });
        } else {
            apply_match_arms.push(quote!{
                ( &mut #name #variant_specifier #left)  => {
                    while let Some(element) = ctx.next_path_element(seq)? {
                        match element {
                            #(#apply_fn_field_handlers)* 
                            _ =>  ctx.skip_value(seq)?
                        }
                    }
                }
            });
        }
    }

    Ok((diff_match_arms, apply_match_arms))
}

fn generate(
    input: &syn::DeriveInput,
    struct_args: args::SerdeDiffStructArgs,
    target_type: Option<syn::Type>,
) -> Result<proc_macro::TokenStream, proc_macro::TokenStream> {

    use syn::Data;
    let mut diff_match_arms = vec![];
    let mut apply_match_arms = vec![];

    let has_variants = match &input.data {
        Data::Enum(e) => {
            for matching in &[true, false] {
                for v in &e.variants {
                    let (diff, apply) = generate_arms(&struct_args.ident, Some(&v.ident), &v.fields, *matching)?;
                    diff_match_arms.extend(diff);
                    apply_match_arms.extend(apply);
                }
            }
            true
        }
        Data::Struct(s) => {
            let matching = true;
            let (diff, apply) = generate_arms(&struct_args.ident, None, &s.fields, matching)?;
            diff_match_arms.extend(diff);
            apply_match_arms.extend(apply);
            false
        }
        _ => {unreachable!("Unhandled Type in Enum")},
    };

    // Generate the SerdeDiff::diff function for the type
    let diff_fn = if let Some(ref ty) = target_type {
        quote! {
            fn diff<'a, S: serde_diff::_serde::ser::SerializeSeq>(&self, ctx: &mut serde_diff::DiffContext<'a, S>, other: &Self) -> Result<bool, S::Error> {
                std::convert::Into::<#ty>::into(std::clone::Clone::clone(self))
                    .diff(ctx, &std::convert::Into::<#ty>::into(std::clone::Clone::clone(other)))
            }
        }
    } else {
        quote! {
            fn diff<'a, S: serde_diff::_serde::ser::SerializeSeq>(&self, ctx: &mut serde_diff::DiffContext<'a, S>, other: &Self) -> Result<bool, S::Error> {
                let mut __changed__ = false;
                match (self, other) {
                    #(#diff_match_arms)*
                }
                Ok(__changed__)
            }
        }
    };

    
    // Generate the SerdeDiff::apply function for the type
    //TODO: Consider using something like the phf crate to avoid a string compare across field names,
    // or consider having the user manually tag their data with a number similar to protobuf
    let apply_fn = if let Some(ref ty) = target_type {
        quote! {
            fn apply<'de, A>(
                &mut self,
                seq: &mut A,
                ctx: &mut serde_diff::ApplyContext,
            ) -> Result<bool, <A as serde_diff::_serde::de::SeqAccess<'de>>::Error>
            where
                A: serde_diff::_serde::de::SeqAccess<'de>, {
                    let mut converted = std::convert::Into::<#ty>::into(std::clone::Clone::clone(self));
                    let result = converted.apply(seq, ctx);
                    *self = std::convert::From::<#ty>::from(converted);
                    result
            }
        }
    } else {
        if has_variants {
            quote! {
                fn apply<'de, A>(
                    &mut self,
                    seq: &mut A,
                    ctx: &mut serde_diff::ApplyContext,
                ) -> Result<bool, <A as serde_diff::_serde::de::SeqAccess<'de>>::Error>
                where
                    A: serde_diff::_serde::de::SeqAccess<'de>, {
                    let mut __changed__ = false;
                    match (self, ctx.next_path_element(seq)?) {
                        (this, Some(serde_diff::DiffPathElementValue::FullEnumVariant)) => {
                            ctx.read_value(seq, this)?;
                            __changed__ = true;
                        }
                        #(#apply_match_arms)*
                        _ => ctx.skip_value(seq)?,
                    }
                    Ok(__changed__)
                }
            }
        } else {
            quote! {
                fn apply<'de, A>(
                    &mut self,
                    seq: &mut A,
                    ctx: &mut serde_diff::ApplyContext,
                ) -> Result<bool, <A as serde_diff::_serde::de::SeqAccess<'de>>::Error>
                where
                    A: serde_diff::_serde::de::SeqAccess<'de>, {
                    let mut __changed__ = false;
                    match (self) {
                        #(#apply_match_arms)*
                        _ => ctx.skip_value(seq)?,
                    }
                    Ok(__changed__)
                }
            }
        }
    };

    // Generate the impl block with the diff and apply functions within it
    let struct_name = &struct_args.ident;
    let generics = &struct_args.generics.params;
    let where_clause =  &struct_args.generics.where_clause;
    let diff_impl = quote! {
        impl <#generics> serde_diff::SerdeDiff for #struct_name < #generics> #where_clause {
            #diff_fn
            #apply_fn
        }
    };
    return Ok(proc_macro::TokenStream::from(quote! {
        #diff_impl
    }));
}

fn generate_opaque(
    _input: &syn::DeriveInput,
    struct_args: args::SerdeDiffStructArgs,
) -> proc_macro::TokenStream {
    let struct_name = &struct_args.ident;
    let diff_impl = quote! {
        impl serde_diff::SerdeDiff for #struct_name {
            fn diff<'a, S: serde_diff::_serde::ser::SerializeSeq>(&self, ctx: &mut serde_diff::DiffContext<'a, S>, other: &Self) -> Result<bool, S::Error> {
                if self != other {
                    ctx.save_value(other)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            fn apply<'de, A>(
                &mut self,
                seq: &mut A,
                ctx: &mut serde_diff::ApplyContext,
            ) -> Result<bool, <A as serde_diff::_serde::de::SeqAccess<'de>>::Error>
            where
                A: serde_diff::_serde::de::SeqAccess<'de>, {
                    ctx.read_value(seq, self)
            }
        }
    };

    return proc_macro::TokenStream::from(quote! {
        #diff_impl
    });
}

// Adapted from serde's internal `parse_lit_into_ty` function (with the chain of helper functions directly cargo culted over)
fn parse_string_to_type(s: String, span: proc_macro2::Span) -> Result<syn::Type, syn::Error> {
    let lit = syn::LitStr::new(&s, span);
    let tokens = spanned_tokens(&lit)?;
    syn::parse2(tokens)
}

fn spanned_tokens(s: &syn::LitStr) -> syn::parse::Result<proc_macro2::TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan_token_stream(stream, s.span()))
}

fn respan_token_stream(stream: proc_macro2::TokenStream, span: proc_macro2::Span) -> proc_macro2::TokenStream {
    stream
        .into_iter()
        .map(|token| respan_token_tree(token, span))
        .collect()
}

fn respan_token_tree(mut token: proc_macro2::TokenTree, span: proc_macro2::Span) -> proc_macro2::TokenTree {
    if let proc_macro2::TokenTree::Group(g) = &mut token {
        *g = proc_macro2::Group::new(g.delimiter(), respan_token_stream(g.stream(), span));
    }
    token.set_span(span);
    token
}

