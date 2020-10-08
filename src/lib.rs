mod codegen;
pub mod format;
pub mod path;
mod reference;
pub mod spec;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub use self::{
    codegen::CodeGen,
    reference::Reference,
    spec::{OperationVerb, ResolvedSchema, Spec},
};
