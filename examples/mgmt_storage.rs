// cargo run --example mgmt_storage
// https://github.com/Azure/azure-rest-api-specs/tree/master/specification/storage/resource-manager

use autorust_codegen::*;

fn main() -> Result<()> {
    let api_version = "2020-08-01-preview";
    let output_folder = "../azure-sdk-for-rust/rest/mgmt_storage/2020-08-01-preview/src/";
    let input_files = [
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/preview/2020-08-01-preview/storage.json",
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/preview/2020-08-01-preview/blob.json",
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/preview/2020-08-01-preview/file.json",
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/preview/2020-08-01-preview/queue.json",
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/preview/2020-08-01-preview/table.json",
    ];
    run(Config {
        api_version: Some(api_version.to_owned()),
        output_folder: output_folder.into(),
        input_files: input_files.iter().map(Into::into).collect(),
    })?;

    let api_version = "2019-06-01";
    let output_folder = "../azure-sdk-for-rust/rest/mgmt_storage/2019-06-01/src/";
    let input_files = [
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/stable/2019-06-01/storage.json",
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/stable/2019-06-01/blob.json",
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/stable/2019-06-01/file.json",
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/stable/2019-06-01/queue.json",
        "../azure-rest-api-specs/specification/storage/resource-manager/Microsoft.Storage/stable/2019-06-01/table.json",
    ];
    run(Config {
        api_version: Some(api_version.to_owned()),
        output_folder: output_folder.into(),
        input_files: input_files.iter().map(Into::into).collect(),
    })?;

    Ok(())
}
