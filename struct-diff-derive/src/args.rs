
use darling::FromField;
//use quote::quote;
use darling::FromDeriveInput;

// Metadata from the struct's type annotation
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(struct_diff))]
pub struct StructDiffStructArgs {
    pub ident: syn::Ident,
}

// We support multiple distinct inspect annotations (i.e. diff_copy, diff_clone, diff_custom)
// Each distinct type will have a struct for capturing the metadata. These metadata structs
// must implement this trait
pub trait StructDiffFieldArgs {
    fn ident(&self) -> &Option<syn::Ident>;
    fn ty(&self) -> &syn::Type;
    fn skip(&self) -> bool;
}

//
// Copy field handling
//
#[derive(Debug, FromField, Clone)]
#[darling(attributes(diffable))]
pub struct StructDiffFieldArgsCopy {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    skip: bool
}

impl StructDiffFieldArgs for StructDiffFieldArgsCopy {
    fn ident(&self) -> &Option<syn::Ident> {
        &self.ident
    }
    fn ty(&self) -> &syn::Type {
        &self.ty
    }
    fn skip(&self) -> bool {
        self.skip
    }
}


//
// Clone field handling
//
#[derive(Debug, FromField, Clone)]
#[darling(attributes(diffable))]
pub struct StructDiffFieldArgsClone {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    skip: bool
}

impl StructDiffFieldArgs for StructDiffFieldArgsClone {
    fn ident(&self) -> &Option<syn::Ident> {
        &self.ident
    }
    fn ty(&self) -> &syn::Type {
        &self.ty
    }
    fn skip(&self) -> bool {
        self.skip
    }
}


//
// Custom field handling
//
#[derive(Debug, FromField, Clone)]
#[darling(attributes(diffable))]
pub struct StructDiffFieldArgsCustom {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    skip: bool
}

impl StructDiffFieldArgs for StructDiffFieldArgsCustom {
    fn ident(&self) -> &Option<syn::Ident> {
        &self.ident
    }
    fn ty(&self) -> &syn::Type {
        &self.ty
    }
    fn skip(&self) -> bool {
        self.skip
    }
}