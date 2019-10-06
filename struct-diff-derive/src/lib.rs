
extern crate proc_macro;

mod args;

use quote::quote;

#[proc_macro_derive(Diffable, attributes(diffable))]
pub fn inspect_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    use darling::FromDeriveInput;

    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_args = args::StructDiffStructArgs::from_derive_input(&input).unwrap();
    let parsed_fields = parse_fields(&input);
    generate(&input, struct_args, parsed_fields)
}

fn handle_inspect_type<
    FieldArgsT: darling::FromField + args::StructDiffFieldArgs + Clone,
    //ArgsT: From<FieldArgsT> + ToTokens,
>(
    parsed_field: &mut Option<ParsedField>,
    f: &syn::Field,
    //default_render_trait: proc_macro2::TokenStream,
    //arg_type: proc_macro2::TokenStream,
)
{
    //TODO: Improve error message
    if parsed_field.is_some() {
        panic!(
            "Too many inspect attributes on a single member {:?}",
            f.ident
        );
    }

    let field_args = FieldArgsT::from_field(&f).unwrap();

    if field_args.skip() {
        *parsed_field = Some(ParsedField {
            //skip: true
        });

        return;
    }


    *parsed_field = Some(ParsedField {
        //skip: true
    });
}

fn try_handle_inspect_type<
    FieldArgsT: darling::FromField + args::StructDiffFieldArgs + Clone,
    //ArgsT: From<FieldArgsT> + ToTokens,
>(
    parsed_field: &mut Option<ParsedField>,
    f: &syn::Field,
    path: &syn::Path,
    //default_render_trait: proc_macro2::TokenStream,
    //arg_type: proc_macro2::TokenStream,
) {
    if f.attrs.iter().find(|x| x.path == *path).is_some() {
        handle_inspect_type::<FieldArgsT/*, ArgsT*/>(parsed_field, &f/*, default_render_trait, arg_type*/);
    }
}


fn handle_inspect_types(parsed_field: &mut Option<ParsedField>, f: &syn::Field) {
    // These are effectively constants
    #[allow(non_snake_case)]
    let STRUCT_DIFF_COPY_PATH = syn::parse2::<syn::Path>(quote!(diff_copy)).unwrap();
    #[allow(non_snake_case)]
    let STRUCT_DIFF_CLONE_PATH = syn::parse2::<syn::Path>(quote!(diff_clone)).unwrap();
    #[allow(non_snake_case)]
    let STRUCT_DIFF_CUSTOM_PATH = syn::parse2::<syn::Path>(quote!(diff_custom)).unwrap();

    // We must check every trait
    try_handle_inspect_type::<args::StructDiffFieldArgsCopy>(
        parsed_field,
        f,
        &STRUCT_DIFF_COPY_PATH,
        //quote!(imgui_inspect::InspectRenderSlider),
        //quote!(imgui_inspect::InspectArgsSlider),
    );

    try_handle_inspect_type::<args::StructDiffFieldArgsClone>(
        parsed_field,
        f,
        &STRUCT_DIFF_CLONE_PATH,
        //quote!(imgui_inspect::InspectRenderDefault),
        //quote!(imgui_inspect::InspectArgsDefault),
    );
    try_handle_inspect_type::<args::StructDiffFieldArgsCustom>(
        parsed_field,
        f,
        &STRUCT_DIFF_CUSTOM_PATH,
        //quote!(imgui_inspect::InspectRenderDefault),
        //quote!(imgui_inspect::InspectArgsDefault),
    );
}

fn parse_fields(input: &syn::DeriveInput) -> Vec<ParsedField> {

    use syn::Data;
    use syn::Fields;

    match input.data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Parse the fields
                    let parsed_fields: Vec<_> = fields
                        .named
                        .iter()
                        .map(|f| {
                            let mut parsed_field: Option<ParsedField> = None;

                            handle_inspect_types(&mut parsed_field, &f);

                            if parsed_field.is_none() {
                                handle_inspect_type::<args::StructDiffFieldArgsCopy>(
                                    &mut parsed_field,
                                    &f,
                                    //quote!(imgui_inspect::InspectRenderDefault),
                                    //quote!(imgui_inspect::InspectArgsDefault),
                                );
                                //parsed_field = Some(ParsedField {});
                            }

                            // We expect the previous code to either successfully parse an attribute
                            // and set parsed_field to a non-none value or panic with a descriptive
                            // error message
                            parsed_field.unwrap()
                        })
                        .collect();

                    parsed_fields
                }
                //Fields::Unit => ,
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }
}

struct ParsedField {
    //render: proc_macro2::TokenStream,
    //render_mut: proc_macro2::TokenStream,
}

fn generate(
    _input: &syn::DeriveInput,
    _struct_args: args::StructDiffStructArgs,
    _parsed_fields: Vec<ParsedField>,
) -> proc_macro::TokenStream {
    return proc_macro::TokenStream::from(quote! {})
}