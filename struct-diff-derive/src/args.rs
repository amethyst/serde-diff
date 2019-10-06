
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
#[darling(attributes(diffable))]
pub struct StructDiffFieldArgs {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    skip: bool,

    #[darling(default)]
    clone: bool,

    #[darling(default)]
    copy: bool,

    #[darling(default)]
    custom: bool,

    #[darling(default)]
    diff_type: Option<syn::Path>
}

impl StructDiffFieldArgs {
    pub fn ident(&self) -> &Option<syn::Ident> {
        return &self.ident
    }
    pub fn ty(&self) -> &syn::Type {
        return &self.ty
    }
    pub fn skip(&self) -> bool {
        return self.skip
    }
    pub fn copy(&self) -> bool {
        return self.copy
    }
    pub fn custom(&self) -> bool {
        return self.custom
    }
    pub fn diff_type(&self) -> &Option<syn::Path> {
        return &self.diff_type
    }
}