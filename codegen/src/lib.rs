pub mod cargo_toml;
mod codegen;
pub mod config_parser;
pub mod lib_rs;
pub mod path;
mod reference;
pub mod spec;
mod status_codes;
pub use self::{
    codegen::{create_mod, CodeGen},
    reference::Reference,
    spec::{OperationVerb, ResolvedSchema, Spec},
};
use proc_macro2::TokenStream;
use snafu::{ResultExt, Snafu};
use std::{
    fs::{self, File},
    io::prelude::*,
    path::PathBuf,
};
#[macro_use]
extern crate lazy_static;

pub type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not create output directory {}: {}", directory.display(), source))]
    CreateOutputDirectory {
        directory: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not create file {}: {}", file.display(), source))]
    CreateFile {
        file: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not write file {}: {}", file.display(), source))]
    WriteFile {
        file: PathBuf,
        source: std::io::Error,
    },
    CodeGenError {
        source: codegen::Error,
    },
    PathError {
        source: path::Error,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub input_files: Vec<PathBuf>,
    pub output_folder: PathBuf,
    pub api_version: Option<String>,
}

pub fn run(config: Config) -> Result<()> {
    let directory = &config.output_folder;
    fs::create_dir_all(directory).context(CreateOutputDirectory { directory })?;
    let cg = &CodeGen::new(config.clone()).context(CodeGenError)?;

    // create models from schemas
    let models = cg.create_models().context(CodeGenError)?;
    let models_path = path::join(&config.output_folder, "models.rs").context(PathError)?;
    write_file(&models_path, &models)?;

    // create api client from operations
    let operations = cg.create_operations().context(CodeGenError)?;
    let operations_path = path::join(&config.output_folder, "operations.rs").context(PathError)?;
    write_file(&operations_path, &operations)?;

    if let Some(api_version) = &config.api_version {
        let operations = create_mod(api_version);
        let operations_path = path::join(&config.output_folder, "mod.rs").context(PathError)?;
        write_file(&operations_path, &operations)?;
    }
    Ok(())
}

pub fn write_file<P: Into<PathBuf>>(file: P, tokens: &TokenStream) -> Result<()> {
    let file: PathBuf = file.into();
    println!("writing file {}", &file.display());
    let code = tokens.to_string();
    let mut buffer = File::create(&file).context(CreateFile { file: file.clone() })?;
    buffer.write_all(&code.as_bytes()).context(WriteFile { file })?;
    Ok(())
}
