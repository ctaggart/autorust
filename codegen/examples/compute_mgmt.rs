// cargo run --example compute_mgmt
// https://github.com/Azure/azure-rest-api-specs/blob/master/specification/compute/resource-manager

use autorust_codegen::{
    config_parser::{to_api_version, to_feature_name, to_mod_name},
    *,
};

fn main() -> Result<()> {
    let md = "../azure-rest-api-specs/specification/compute/resource-manager/readme.md";
    let output_folder = "../azure-sdk-for-rust/services/mgmt/compute/src";

    let packages = config_parser::parse_configurations_from_autorest_config_file(md.into());
    for package in packages {
        println!("{}", &package.tag);
        if let Some(api_version) = to_api_version(&package.tag) {
            println!("  {}", api_version);
            let feature_name = &to_feature_name(&package.tag);
            let mod_name = &to_mod_name(feature_name);
            println!("  {}", feature_name);
            println!("  {}", mod_name);
            let mod_output_folder = path::join(output_folder, mod_name)?;
            println!("  {:?}", mod_output_folder);
            // for input_file in &package.input_files {
            //     println!("  {}", input_file);
            // }
            let input_files: Vec<_> = package
                .input_files
                .iter()
                .map(|input_file| path::join(md, input_file).unwrap())
                .collect();
            for input_file in &input_files {
                println!("  {:?}", input_file);
            }
            run(Config {
                api_version: Some(api_version),
                output_folder: mod_output_folder.into(),
                input_files,
            })?;
        }
    }

    Ok(())
}
