// cargo test --test azure_rest_api_specs
// These tests require cloning azure-rest-api-specs.
// git clone git@github.com:Azure/azure-rest-api-specs.git ../azure-rest-api-specs

use autorust_codegen::*;

#[test]
fn ref_files() -> Result<()> {
    let api = &read_api_file("../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json")?;
    let ref_files: Vec<String> = get_ref_files(api)?.into_iter().collect();
    assert_eq!(
        ref_files,
        vec!["../../../../../common-types/resource-management/v1/types.json".to_owned(),]
    );
    Ok(())
}
