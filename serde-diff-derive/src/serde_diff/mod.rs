extern crate proc_macro;

mod args;

use quote::quote;

/// Reads in all tokens for the struct having the
pub fn macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use darling::FromDeriveInput;

    // Parse the struct
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_args = args::SerdeDiffStructArgs::from_derive_input(&input).unwrap();

    if struct_args.opaque {
        generate_opaque(&input, struct_args)
    } else {
        let parsed_fields = parse_fields(&input);
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
            return proc_macro::TokenStream::from(darling::Error::multiple(errors).write_errors());
        }

        // Go ahead and generate the code
        generate(&input, struct_args, ok_fields)
    }
}

/// Called per field to parse and verify it
fn parse_field(f: &syn::Field) -> Result<ParsedField, darling::Error> {
    use darling::FromField;
    let field_args = args::SerdeDiffFieldArgs::from_field(&f)?;

    Ok(ParsedField { field_args })
}

/// Walks all fields, parsing and verifying them
fn parse_fields(input: &syn::DeriveInput) -> Vec<Result<ParsedField, darling::Error>> {
    use syn::Data;
    use syn::Fields;

    match input.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => fields.named.iter().map(|f| parse_field(&f)).collect(),
                _ => unimplemented!(), // Fields::Unnamed, Fields::Unit currently not supported
            }
        }
        _ => unimplemented!(), // Data::Enum, Data::Union currently not supported
    }
}

/// Parsed metadata for a field
#[derive(Debug)]
struct ParsedField {
    field_args: args::SerdeDiffFieldArgs,
}

/// Takes the parsed input and produces implementation of the macro
fn generate(
    _input: &syn::DeriveInput,
    struct_args: args::SerdeDiffStructArgs,
    parsed_fields: Vec<ParsedField>,
) -> proc_macro::TokenStream {
    //let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

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

        if pf.field_args.opaque() {
            diff_fn_field_handlers.push(quote! {
                {
                    ctx.push_field(#ident_as_str);
                    if self.#ident != other.#ident {
                        ctx.save_value(&other.#ident)?;
                        __changed__ |= true;
                    }
                    ctx.pop_path_element()?;
                }
            });
        } else {
            diff_fn_field_handlers.push(quote! {
                {
                    {
                        match ctx.field_path_mode() {
                            serde_diff::FieldPathMode::Name => ctx.push_field(#ident_as_str),
                            serde_diff::FieldPathMode::Index => ctx.push_field_index(#field_idx),
                        }
                        __changed__ |= <#ty as serde_diff::SerdeDiff>::diff(&self.#ident, ctx, &other.#ident)?;
                        ctx.pop_path_element()?;
                    }
                }
            });
        }
    }

    // Generate the SerdeDiff::diff function for the type
    let diff_fn = quote! {
        fn diff<'a, S: serde_diff::_serde::ser::SerializeSeq>(&self, ctx: &mut serde_diff::DiffContext<'a, S>, other: &Self) -> Result<bool, S::Error> {
            let mut __changed__ = false;
            #(#diff_fn_field_handlers)*
            Ok(__changed__)
        }
    };

    // This will hold a bit of code per-field that call apply on that field
    let mut apply_fn_field_handlers = vec![];
    for (field_idx, pf) in parsed_fields.iter().enumerate() {
        // Skip fields marked as #[serde_diff(skip)]
        if pf.field_args.skip() {
            continue;
        }

        let ident = pf.field_args.ident().clone();
        let ident_as_str = quote!(#ident).to_string();
        let ty = pf.field_args.ty();
        let field_idx = field_idx as u16;

        if pf.field_args.opaque() {
            apply_fn_field_handlers.push(quote!(
                serde_diff::DiffPathElementValue::FieldIndex(#field_idx) => __changed__ |= ctx.read_value(seq, &mut self.#ident)?,
                serde_diff::DiffPathElementValue::Field(field_path) if field_path.as_ref() == #ident_as_str => __changed__ |= ctx.read_value(seq, &mut self.#ident)?,
            ));
        } else {
            apply_fn_field_handlers.push(quote!(
                serde_diff::DiffPathElementValue::FieldIndex(#field_idx) => __changed__ |= <#ty as serde_diff::SerdeDiff>::apply(&mut self.#ident, seq, ctx)?,
                serde_diff::DiffPathElementValue::Field(field_path) if field_path.as_ref() == #ident_as_str => __changed__ |= <#ty as serde_diff::SerdeDiff>::apply(&mut self.#ident, seq, ctx)?,
            ));
        }
    }

    // Generate the SerdeDiff::apply function for the type
    //TODO: Consider using something like the phf crate to avoid a string compare across field names,
    // or consider having the user manually tag their data with a number similar to protobuf
    let apply_fn = quote! {
        fn apply<'de, A>(
            &mut self,
            seq: &mut A,
            ctx: &mut serde_diff::ApplyContext,
        ) -> Result<bool, <A as serde_diff::_serde::de::SeqAccess<'de>>::Error>
        where
            A: serde_diff::_serde::de::SeqAccess<'de>, {
            let mut __changed__ = false;
            while let Some(element) = ctx.next_path_element(seq)? {
                match element {
                    #(#apply_fn_field_handlers)*
                    _ => ctx.skip_value(seq)?,
                }
            }
            Ok(__changed__)
        }
    };

    // Generate the impl block with the diff and apply functions within it
    let struct_name = &struct_args.ident;
    let diff_impl = quote! {
        impl serde_diff::SerdeDiff for #struct_name {
            #diff_fn
            #apply_fn
        }
    };

    return proc_macro::TokenStream::from(quote! {
        #diff_impl
    });
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
