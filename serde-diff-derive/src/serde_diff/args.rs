use darling::{FromDeriveInput, FromField};

/// Metadata from the struct's type annotation
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(serde_diff))]
pub struct SerdeDiffStructArgs {
    /// Name of the struct
    pub ident: syn::Ident,
    /// Whether the struct is opaque or not
    #[darling(default)]
    pub opaque: bool,
    /// If specified, the struct we will convert to before performing diff operations
    #[darling(default)]
    pub target: Option<String>,

    pub generics: syn::Generics,
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

    /// If true, simple diff should be generated inline
    #[darling(default)]
    opaque: bool,
}

impl SerdeDiffFieldArgs {
    /// Name of the field
    pub fn ident(&self) -> &Option<syn::Ident> {
        &self.ident
    }

    /// Type of the field
    pub fn ty(&self) -> &syn::Type {
        &self.ty
    }

    /// If true, simple diff should be generated inline
    pub fn skip(&self) -> bool {
        self.skip
    }

    /// If true, this field should be ignored

    pub fn opaque(&self) -> bool {
        self.opaque
    }
}
