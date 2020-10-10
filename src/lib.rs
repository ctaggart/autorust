mod codegen;
mod reference;
mod literate_config_parser;
pub mod format;
pub mod path;
pub mod spec;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub use self::{
    codegen::CodeGen,
    reference::Reference,
    spec::{OperationVerb, ResolvedSchema, Spec},
};
