mod cli;

use autorust_codegen::{format, path, CodeGen, Result};
use proc_macro2::TokenStream;
use std::{
    fs::{self, File},
    io::prelude::*,
    path::Path,
};

fn main() -> Result<()> {
    let config = cli::Config::try_new()?;
    fs::create_dir_all(config.output_folder())?;
    let cg = &CodeGen::from_files(config.input_files())?;

    // create models from schemas
    let models = cg.create_models()?;
    let models_path = path::join(config.output_folder(), "models.rs")?;
    write_file(&models_path, &models)?;

    // create api client from operations
    let client = cg.create_client()?;
    let client_path = path::join(&config.output_folder(), "client.rs")?;
    write_file(&client_path, &client)?;
    Ok(())
}

fn write_file<P: AsRef<Path>>(path: P, tokens: &TokenStream) -> Result<()> {
    println!("writing file {}", path.as_ref().display());
    let code = format::format_code(tokens.to_string());
    let mut buffer = File::create(path)?;
    buffer.write_all(&code.as_bytes())?;
    Ok(())
}
