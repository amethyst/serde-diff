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
    match input.data {
        Data::Struct(..) => {
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
        Data::Enum(..) => {
             if struct_args.opaque {
                 generate_opaque(&input, struct_args)
             } else {
                 // Go ahead and generate the code                 
                 match generate_enum(&input, struct_args) {
                     //Ok(v) => {eprintln!("{}", v); v},
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

fn parse_enum_fields(input: &syn::Fields) -> Vec<Result<ParsedField, darling::Error>> {
    use syn::Fields;
    match input {
        Fields::Named(ref fields) => fields.named.iter().map(|f| parse_field(&f)).collect(),
        Fields::Unnamed(ref fields) => fields.unnamed.iter().map(|f| parse_field(&f)).collect(),
        Fields::Unit => vec![],
    }
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

fn generate_enum_fields_diff(
    _input: &syn::DeriveInput,
    _struct_args: &args::SerdeDiffStructArgs,
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
    let parsed_fields = parse_enum_fields(fields);
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

fn generate_enum(
    input: &syn::DeriveInput,
    struct_args: args::SerdeDiffStructArgs,
) -> Result<proc_macro::TokenStream, proc_macro::TokenStream> {
    let mut diff_match_arms = vec![];
    let mut apply_match_arms = vec![];
    use syn::Data;

    match &input.data {
        Data::Enum(e) => {
            for matching in &[true, false] {
                for v in &e.variants {
                    let name = &struct_args.ident;
                    let variant = &v.ident;                    
                    let parsed_fields = ok_fields(&v.fields)?;
                    let diffs = generate_enum_fields_diff(
                        input,
                        &struct_args,
                        &parsed_fields,
                        *matching,
                    );                    
                    let (left, right) =  enum_fields(&v.fields, false);
                    let variant_as_str = variant.to_string();
                    let left = if *matching {
                        quote! { #name :: #variant #left }
                    } else {
                        quote! {_}
                    };
                    if *matching {
                        diff_match_arms.push(                  
                            quote!{
                                (#left, #name :: #variant #right) => {
                                    ctx.push_variant(#variant_as_str);
                                    #diffs
                                    ctx.pop_path_element()?;
                                }
                            }
                        );
                    } else {
                        diff_match_arms.push(                  
                            quote!{
                                (#left, #name :: #variant #right) => {
                                    ctx.push_full_variant();
                                    ctx.save_value(other)?;
                                    ctx.pop_path_element()?;
                                }
                            }
                        );  
                    }
                    
                    if *matching {
                        let (left, _right) =  enum_fields(&v.fields, true);
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
                        apply_match_arms.push(quote!{
                            ( &mut #name :: #variant #left, Some(serde_diff::DiffPathElementValue::EnumVariant(variant))) if variant == #variant_as_str => {
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
            }
        }
        _ => {unreachable!("Unhandled Type in Enum")},
    }

    // Generate the SerdeDiff::diff function for the type
    let diff_fn = quote! {
        fn diff<'a, S: serde_diff::_serde::ser::SerializeSeq>(&self, ctx: &mut serde_diff::DiffContext<'a, S>, other: &Self) -> Result<bool, S::Error> {
            let mut __changed__ = false;
            match (self, other) {
                #(#diff_match_arms)*
            }
            Ok(__changed__)
        }
    };

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
    };

    // Generate the impl block with the diff and apply functions within it
    let struct_name = &struct_args.ident;
    let diff_impl = quote! {
        impl serde_diff::SerdeDiff for #struct_name {
            #diff_fn
            #apply_fn
        }
    };
    return Ok(proc_macro::TokenStream::from(quote! {
        #diff_impl
    }));
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
