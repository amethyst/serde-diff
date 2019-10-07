
use darling::FromField;
//use quote::quote;
use darling::FromDeriveInput;

// Metadata from the struct's type annotation
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(struct_diff))]
pub struct StructDiffStructArgs {
    pub ident: syn::Ident,
}


//
// Metadata from the struct's field annotations
//
#[derive(Debug, FromField, Clone)]
#[darling(attributes(serde_diffable))]
pub struct StructDiffFieldArgs {
    ident: Option<syn::Ident>,
    //ty: syn::Type,

    #[darling(default)]
    skip: bool,
}

impl StructDiffFieldArgs {
    pub fn ident(&self) -> &Option<syn::Ident> {
        return &self.ident
    }
//    pub fn ty(&self) -> &syn::Type {
//        return &self.ty
//    }
    pub fn skip(&self) -> bool {
        return self.skip
    }
}