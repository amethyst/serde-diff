
extern crate proc_macro;

mod args;

use quote::quote;

#[proc_macro_derive(Diffable, attributes(diffable))]
pub fn inspect_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {

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

fn get_diff_type(field_args: &args::StructDiffFieldArgs) -> syn::Path {
    // This type is the "diff" representation for the given field
    // - Copy/Clone: Will be "T" - i.e. a diff of an f32 is stored as an f32
    // - Custom: By default, tacks "Diff" to the end of the type. This mirrors the behavior
    //   for deriving "Diffable" on a struct. But this can be overridden by setting diff_type
    //   in the field attribute
    field_args.diff_type().clone().unwrap_or_else(|| {
        let ty = field_args.ty();
        let diff_type = if field_args.diff_by_custom() {
            // Handles the case of a custom diff where diff_type is unspecified. Default it to
            // the struct name we would produce for the given type
            let s = format!("{}Diff", quote!(#ty));
            let ty = quote::format_ident!("{}", s);
            quote!(#ty)
        } else {
            // Handles the copy/clone case. Just return the type
            quote!(#ty)
        };

        syn::parse2::<syn::Path>(quote!{#diff_type}).unwrap()
    })
}

fn parse_field(
    f: &syn::Field,
) -> Result<ParsedField, darling::Error>
{
    //TODO: Unwrapping is less clear, figure out how to return
    use darling::FromField;
    let field_args = args::StructDiffFieldArgs::from_field(&f)?;
    let diff_type = get_diff_type(&field_args);

    Ok(ParsedField {
        field_args,
        diff_type
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
    //render: proc_macro2::TokenStream,
    //render_mut: proc_macro2::TokenStream,
    field_args: args::StructDiffFieldArgs,
    diff_type: syn::Path
}

fn generate(
    input: &syn::DeriveInput,
    struct_args: args::StructDiffStructArgs,
    parsed_fields: Vec<ParsedField>,
) -> proc_macro::TokenStream
{
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut diff_struct_fields = vec![];
    for pf in &parsed_fields {

        let diff_ident = pf.field_args.ident().clone();
        let diff_type = pf.diff_type.clone();

        diff_struct_fields.push(quote!{
            #diff_ident: Option<#diff_type>
        })
    }

    let struct_name = &struct_args.ident;
    let diff_struct_name = quote::format_ident!("{}Diff", struct_name);

    let diff_struct = quote! {
        //TODO: Need a way to control what is derived here (i.e. Debug, Serialize, etc.)
        #[derive(Default, Debug)]
        struct #diff_struct_name {
            #(#diff_struct_fields),*
        }
    };

    let mut diff_fn_field_handlers = vec![];
    for pf in &parsed_fields {

        let ident = pf.field_args.ident().clone();
        let ty = pf.field_args.ty().clone();

        let diffable_trait = if pf.field_args.diff_by_copy() {
            quote::format_ident!("DiffableByCopy")
        } else if pf.field_args.diff_by_clone() {
            quote::format_ident!("DiffableByClone")
        } else if pf.field_args.diff_by_custom() {
            quote::format_ident!("DiffableByCustom")
        } else {
            panic!("Expected field to be diffable by copy, clone, or custom");
        };

        diff_fn_field_handlers.push(quote!{
            {
                {
                    let member_diff = <#ty as #diffable_trait<_, _>>::diff(&old.#ident, &new.#ident);
                    if member_diff.is_some() {
                        struct_diff.#ident = member_diff;
                        has_change = true;
                    }
                }
            }
        });
    }

    let diff_fn = quote! {
        fn diff(old: &#struct_name, new: &#struct_name) -> Option<#diff_struct_name> {
            let mut struct_diff = #diff_struct_name::default();
            let mut has_change = false;

            #(#diff_fn_field_handlers)*

//          {
//              {
//                  let member_diff = <Vec<String> as DiffableByCustom<_, _>>::diff(&old.string_list, &new.string_list);
//                  if member_diff.is_some() {
//                      struct_diff.string_list = member_diff;
//                      has_change = true;
//                  }
//              }
//          }

            if has_change {
                Some(struct_diff)
            } else {
                None
            }
        }
    };

    let mut apply_fn_field_handlers = vec![];
    for pf in &parsed_fields {
        let ident = pf.field_args.ident().clone();

        let handler = if pf.field_args.diff_by_copy() {
            quote!{
                if let Some(diff) = diff {
                    if let Some(#ident) = diff.#ident {
                        target.#ident = #ident;
                    }
                }
            }
        } else if pf.field_args.diff_by_clone() {
            quote!{
                if let Some(diff) = diff {
                    if let Some(#ident) = &diff.#ident {
                        target.#ident = #ident.clone();
                    }
                }
            }

        } else {
            unimplemented!()
        };

        apply_fn_field_handlers.push(handler);
    }

    let apply_fn = quote! {
        fn apply(diff: &Option<#diff_struct_name>, target: &mut #struct_name) {
            #(#apply_fn_field_handlers)*
        }
    };


    let diff_impl = quote! {
        impl DiffableByCustom<#struct_name, Option<#diff_struct_name>> for #struct_name {
            #diff_fn
            #apply_fn
        }
    };

    return proc_macro::TokenStream::from(quote! {
        #diff_struct
        #diff_impl
    })
}