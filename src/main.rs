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
    for input_file in config.input_files() {
        let cg = &CodeGen::from_file(input_file)?;

        // create models from schemas
        let models = cg.create_models()?;
        let models_path = path::join(false, config.output_folder(), "models.rs")?;
        write_file(&models, &models_path);

        // create api client from operations
        let client = cg.create_client()?;
        let client_path = path::join(false, &config.output_folder(), "client.rs")?;
        write_file(&client, &client_path);
    }
    Ok(())
}

pub fn write_file(tokens: &TokenStream, path: &Path) {
    println!("writing file {}", path.display());
    let code = format::format_code(tokens.to_string());
    let mut buffer = File::create(path).unwrap();
    buffer.write_all(&code.as_bytes()).unwrap();
}
