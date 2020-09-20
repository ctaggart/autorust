use std::process::exit;

use autorust_codegen::{create_api_client, create_client, read_api_file, write_file, Result};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("path to spec required");
        exit(1);
    }
    let path = &args[1];
    let api = &read_api_file(path)?;

    // TODO combine into single file

    // create model from definitions
    let model = create_client(api);
    write_file(&model, "model.rs");

    // create api client from operations
    let client = create_api_client(api);
    write_file(&client, "client.rs");

    Ok(())
}
