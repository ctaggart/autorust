mod codegen;
pub mod format;
pub mod path;
mod reference;
pub mod spec;
pub use self::{
    codegen::CodeGen,
    reference::Reference,
    spec::{OperationVerb, ResolvedSchema, Spec},
};
use proc_macro2::TokenStream;
use std::{
    fs::{self, File},
    io::prelude::*,
    path::{Path, PathBuf},
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub input_files: Vec<PathBuf>,
    pub output_folder: PathBuf,
    pub api_version: Option<String>,
}

pub fn run(config: Config) -> Result<()> {
    fs::create_dir_all(&config.output_folder)?;
    let cg = &CodeGen::new(config.clone())?;

    // create models from schemas
    let models = cg.create_models()?;
    let models_path = path::join(&config.output_folder, "models.rs")?;
    write_file(&models_path, &models)?;

    // create api client from operations
    let operations = cg.create_operations()?;
    let operations_path = path::join(&config.output_folder, "operations.rs")?;
    write_file(&operations_path, &operations)?;
    Ok(())
}

fn write_file<P: AsRef<Path>>(path: P, tokens: &TokenStream) -> Result<()> {
    println!("writing file {}", path.as_ref().display());
    let code = format::format_code(tokens.to_string());
    let mut buffer = File::create(path)?;
    buffer.write_all(&code.as_bytes())?;
    Ok(())
}
