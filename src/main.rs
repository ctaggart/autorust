use autorust_codegen::{
    create_api_client, create_client, new_app, new_config, write_file, Result, Spec,
};

fn main() -> Result<()> {
    let arg_matches = new_app().get_matches();
    let config = new_config(&arg_matches)?;
    for input_file in &config.input_files {
        let spec = &Spec::read_file(input_file)?;

        // TODO naming of files
        // TODO may be combine into single file

        // create model from definitions
        let model = create_client(spec);
        write_file(&model, "model.rs");

        // create api client from operations
        let client = create_api_client(spec);
        write_file(&client, "client.rs");
    }
    Ok(())
}
