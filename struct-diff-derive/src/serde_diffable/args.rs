use darling::{FromField, FromDeriveInput};

/// Metadata from the struct's type annotation
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(serde_diff))]
pub struct SerdeDiffStructArgs {
    /// Name of the struct
    pub ident: syn::Ident,
}

/// Metadata from the struct's field annotations
#[derive(Debug, FromField, Clone)]
#[darling(attributes(serde_diff))]
pub struct SerdeDiffFieldArgs {
    /// Name of the field
    ident: Option<syn::Ident>,

    /// Type of the field
    ty: syn::Type,

    /// If true, this field should be ignored
    #[darling(default)]
    skip: bool,
}

impl SerdeDiffFieldArgs {
    /// Name of the field
    pub fn ident(&self) -> &Option<syn::Ident> {
        return &self.ident
    }

    /// Type of the field
    pub fn ty(&self) -> &syn::Type {
       return &self.ty
   }

    /// If true, this field should be ignored
    pub fn skip(&self) -> bool {
        return self.skip
    }
}