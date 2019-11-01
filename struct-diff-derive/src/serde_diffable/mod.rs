
extern crate proc_macro;

mod args;

use quote::quote;

pub fn diffable_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    use darling::FromDeriveInput;

    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_args = args::StructDiffStructArgs::from_derive_input(&input).unwrap();
    let parsed_fields = parse_fields(&input);

    // Check all parsed fields for any errors that may have occurred
    let mut ok_fields : Vec<ParsedField> = vec![];
    let mut errors = vec![];
    for pf in parsed_fields {
        match pf {
            Ok(value) => ok_fields.push(value),
            Err(e) => errors.push(e)
        }
    }

    // If any error occurred, return them all here
    if !errors.is_empty() {
        return proc_macro::TokenStream::from(darling::Error::multiple(errors).write_errors());
    }

    // Go ahead and generate the code
    generate(&input, struct_args, ok_fields)
}

fn parse_field(
    f: &syn::Field,
) -> Result<ParsedField, darling::Error>
{
    //TODO: Unwrapping is less clear, figure out how to return
    use darling::FromField;
    let field_args = args::StructDiffFieldArgs::from_field(&f)?;

    Ok(ParsedField {
        field_args
    })
}

fn parse_fields(input: &syn::DeriveInput) -> Vec<Result<ParsedField, darling::Error>> {

    use syn::Data;
    use syn::Fields;

    match input.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    fields
                        .named
                        .iter()
                        .map(|f| parse_field(&f))
                        .collect()
                }
                //Fields::Unit => ,
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }
}

#[derive(Debug)]
struct ParsedField {
    field_args: args::StructDiffFieldArgs,
}

fn generate(
    _input: &syn::DeriveInput,
    struct_args: args::StructDiffStructArgs,
    parsed_fields: Vec<ParsedField>,
) -> proc_macro::TokenStream
{
    //let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut diff_fn_field_handlers = vec![];
    for pf in &parsed_fields {
        // Skip fields marked as #[serde_diffable(skip)]
        if pf.field_args.skip() {
            continue;
        }

        let ident = pf.field_args.ident().clone();
        let ident_as_str = quote!(#ident).to_string();
        let ty = pf.field_args.ty();

        diff_fn_field_handlers.push(quote!{
            {
                {
                    ctx.push_field(#ident_as_str);
                    <#ty as SerdeDiffable>::diff(&self.#ident, ctx, &other.#ident)?;
                    ctx.pop_path_element();
                }
            }
        });
    }

    let diff_fn = quote! {
        fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error> {
            #(#diff_fn_field_handlers)*
            Ok(())
        }
    };

    let mut apply_fn_field_handlers = vec![];
    for pf in &parsed_fields {
        // Skip fields marked as #[serde_diffable(skip)]
        if pf.field_args.skip() {
            continue;
        }

        let ident = pf.field_args.ident().clone();
        let ident_as_str = quote!(#ident).to_string();
        let ty = pf.field_args.ty();

        apply_fn_field_handlers.push(quote!(
            #ident_as_str => <#ty as SerdeDiffable>::apply(&mut self.#ident, seq, ctx)?,
        ));
    }

    let apply_fn = quote! {
        fn apply<'de, A>(
            &mut self,
            seq: &mut A,
            ctx: &mut ApplyContext,
        ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
        where
            A: de::SeqAccess<'de>, {
            while let Some(DiffPathElementValue::Field(element)) = ctx.next_path_element(seq)? {
                match element.as_ref() {
                    #(#apply_fn_field_handlers)*
                    _ => ctx.skip_value(seq)?,
                }
            }
            Ok(())
        }
    };

    let struct_name = &struct_args.ident;

    let diff_impl = quote! {
        impl SerdeDiffable for #struct_name {
            #diff_fn
            #apply_fn
        }
    };

    return proc_macro::TokenStream::from(quote! {
        #diff_impl
    })
}