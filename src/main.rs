use autorust_codegen::{
    create_client, create_model, new_app, new_config, write_file, CodeGen, Result, Spec,
};

fn main() -> Result<()> {
    let arg_matches = new_app().get_matches();
    let config = new_config(&arg_matches)?;
    for input_file in &config.input_files {
        let spec = Spec::read_file(input_file)?;
        let cg = &CodeGen { spec };

        // TODO naming of files
        // TODO may be combine into single file

        // create model from definitions
        let model = create_model(cg)?;
        write_file(&model, "model.rs");

        // create api client from operations
        let client = create_client(cg)?;
        write_file(&client, "client.rs");
    }
    Ok(())
}
