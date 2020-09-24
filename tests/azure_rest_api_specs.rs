// cargo test --test azure_rest_api_specs
// These tests require cloning azure-rest-api-specs.
// git clone git@github.com:Azure/azure-rest-api-specs.git ../azure-rest-api-specs

use autorust_codegen::*;

#[test]
fn refs_count_security_common() -> Result<()> {
    let api = &read_api_file(
        "../azure-rest-api-specs/specification/security/resource-manager/common/v1/types.json",
    )?;
    let refs = get_refs(api);
    assert_eq!(13, refs.len());
    Ok(())
}

#[test]
fn refs_count_avs() -> Result<()> {
    let api = &read_api_file("../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json")?;
    let refs = get_refs(api);
    assert_eq!(190, refs.len());
    Ok(())
}

#[test]
fn ref_files() -> Result<()> {
    let api = &read_api_file("../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json")?;
    let files = get_ref_files(api)?;
    assert_eq!(1, files.len());
    assert!(files.contains("../../../../../common-types/resource-management/v1/types.json"));
    Ok(())
}

#[test]
fn read_spec_avs() -> Result<()> {
    let spec = &Spec::read_file("../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json")?;
    assert_eq!(2, spec.docs.len());
    assert!(spec.docs.contains_key(
        "../azure-rest-api-specs/specification/common-types/resource-management/v1/types.json"
    ));
    Ok(())
}

#[test]
fn test_resolve_schema_ref() -> Result<()> {
    let file = "../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json";
    let spec = &Spec::read_file(file)?;
    spec.resolve_schema_ref(file, "#/definitions/OperationList")?;
    spec.resolve_schema_ref(
        file,
        "../../../../../common-types/resource-management/v1/types.json#/definitions/ErrorResponse",
    )?;
    Ok(())
}

#[test]
fn test_resolve_parameter_ref() -> Result<()> {
    let file = "../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json";
    let spec = &Spec::read_file(file)?;
    spec.resolve_parameter_ref(file, "../../../../../common-types/resource-management/v1/types.json#/parameters/ApiVersionParameter")?;
    Ok(())
}
