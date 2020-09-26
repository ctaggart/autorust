use autorust_codegen::{
    create_client, create_models, new_app, new_config, path_join, write_file, CodeGen, Result, Spec,
};
use std::fs::create_dir_all;

fn main() -> Result<()> {
    let arg_matches = new_app().get_matches();
    let config = new_config(&arg_matches)?;
    create_dir_all(&config.output_folder)?;
    for input_file in &config.input_files {
        let spec = Spec::read_file(input_file)?;
        let cg = &CodeGen { spec };

        // create models from schemas
        let models = create_models(cg)?;
        let models_path = path_join(false, &config.output_folder, "models.rs")?;
        write_file(&models, &models_path);

        // create api client from operations
        let client = create_client(cg)?;
        let client_path = path_join(false, &config.output_folder, "client.rs")?;
        write_file(&client, &client_path);
    }
    Ok(())
}
