mod cli;
mod codegen;
mod format;
mod path;
mod reference;
mod spec;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub use self::{cli::*, codegen::*, format::*, path::*, reference::*, spec::*};
