// cargo run --example compute_mgmt
// https://github.com/Azure/azure-rest-api-specs/blob/master/specification/compute/resource-manager

use autorust_codegen::*;

fn main() -> Result<()> {
    let api_version = "2020-06-01";
    let output_folder = "../azure-sdk-for-rust/services/compute/mgmt/src/v2020_06_01";
    let input_files = [
        "../azure-rest-api-specs/specification/compute/resource-manager/Microsoft.Compute/stable/2020-06-01/compute.json",
        "../azure-rest-api-specs/specification/compute/resource-manager/Microsoft.Compute/stable/2020-06-01/runCommands.json",
        "../azure-rest-api-specs/specification/compute/resource-manager/Microsoft.Compute/stable/2019-04-01/skus.json",
        "../azure-rest-api-specs/specification/compute/resource-manager/Microsoft.Compute/stable/2020-05-01/disk.json",
        "../azure-rest-api-specs/specification/compute/resource-manager/Microsoft.Compute/stable/2019-12-01/gallery.json",
        "../azure-rest-api-specs/specification/compute/resource-manager/Microsoft.ContainerService/stable/2017-01-31/containerService.json",
    ];
    run(Config {
        api_version: Some(api_version.to_owned()),
        output_folder: output_folder.into(),
        input_files: input_files.iter().map(Into::into).collect(),
    })?;

    Ok(())
}
