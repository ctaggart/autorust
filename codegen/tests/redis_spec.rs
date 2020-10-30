// cargo test --test redis_specs
// These tests require cloning azure-rest-api-specs.
// git clone git@github.com:Azure/azure-rest-api-specs.git ../azure-rest-api-specs

use std::path::PathBuf;
use autorust_codegen::{
    spec::{self, RefString},
    Reference, Spec,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const REDIS_SPEC: &str = "../../azure-rest-api-specs/specification/redis/resource-manager/Microsoft.Cache/stable/2020-06-01/redis.json";
const LINKS_SPEC: &str = "../../azure-rest-api-specs/specification/common-types/resource-management/v1/privatelinks.json";

#[test]
fn test_redis_ref_files() -> Result<()> {
    let api = &spec::openapi::parse(REDIS_SPEC)?;
    let files = spec::openapi::get_ref_files(api);
    println!("{:#?}", files);
    assert_eq!(2, files.len());
    assert!(files.contains("../../../../../common-types/resource-management/v2/types.json"));
    Ok(())
}

#[test]
fn test_redis_read_spec() -> Result<()> {
    let spec = &Spec::read_files(&[REDIS_SPEC])?;
    println!("{:#?}", spec.docs.keys());
    assert_eq!(3, spec.docs.len());
    assert!(spec.docs.contains_key(std::path::Path::new(
        "../../azure-rest-api-specs/specification/common-types/resource-management/v2/types.json"
    )));
    Ok(())
}

#[test]
fn test_links_ref_files() -> Result<()> {
    let api = &spec::openapi::parse(LINKS_SPEC)?;
    let files = spec::openapi::get_ref_files(api);
    println!("{:#?}", files);
    assert_eq!(1, files.len());
    assert!(files.contains("./types.json"));
    Ok(())
}

#[test]
fn test_links_refs_count() -> Result<()> {
    let api = &spec::openapi::parse(LINKS_SPEC)?;
    let refs = spec::get_refs(api);
    assert_eq!(10, refs.len());
    Ok(())
}

// #[test]
// fn test_redis_resolve_all_refs() -> Result<()> {
//     let doc_file = PathBuf::from(REDIS_SPEC);
//     let spec = &Spec::read_files(&[&doc_file])?;
//     for (doc_file, doc) in &spec.docs {
//         let refs = spec::get_refs(doc);
//         for rs in refs {
//             match rs {
//                 RefString::PathItem(_) => {}
//                 RefString::Example(_) => {}
//                 RefString::Parameter(reference) => {
//                     spec.resolve_parameter_ref(&doc_file, Reference::parse(&reference))?;
//                 }
//                 RefString::Schema(reference) => {
//                     spec.resolve_schema_ref(&doc_file, Reference::parse(&reference))?;
//                 }
//             }
//         }
//     }
//     Ok(())
// }
