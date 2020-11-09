// cargo run --example gen_mgmt
// https://github.com/Azure/azure-rest-api-specs/blob/master/specification/compute/resource-manager
use autorust_codegen::{
    self, cargo_toml,
    config_parser::{self, to_api_version, to_mod_name},
    lib_rs, path, Config, PropertyName,
};
use heck::SnakeCase;
use snafu::{OptionExt, ResultExt, Snafu};
use std::{collections::HashSet, fs, path::PathBuf};

const SPEC_FOLDER: &str = "../azure-rest-api-specs/specification";
const OUTPUT_FOLDER: &str = "../azure-sdk-for-rust/services/mgmt";

const ONLY_SERVICES: &[&str] = &[
    // "vmware",
];

const SKIP_SERVICES: &[&str] = &[
    "automation",                 // TODO #81 DataType::File
    "deploymentmanager",          // TODO #80 path parameters
    "deviceprovisioningservices", // TODO #82 certificate_name used as parameter more than once
    "dnc",                        // https://github.com/Azure/azure-rest-api-specs/pull/11578 two ControllerDetails types
    "mixedreality",               // TODO #83 AccountKeyRegenerateRequest not generated
    "netapp",                     // Ident "10minutely"
    "powerplatform",              // https://github.com/Azure/azure-rest-api-specs/pull/11580 incorrect ref & duplicate Operations_List
    "service-map",                // Ident "Ref:machine"
    "servicefabric",              // https://github.com/Azure/azure-rest-api-specs/pull/11581 allOf mistakes and duplicate Operations_List
    "web",                        // TODO #81 DataType::File
];

const SKIP_SERVICE_TAGS: &[(&str, &str)] = &[
    ("azureactivedirectory", "package-preview-2020-07"),
    ("resources", "package-policy-2020-03"),
    ("resources", "package-policy-2020-09"), // SchemaNotFound { ref_key: RefKey { file_path: "../azure-rest-api-specs/specification/resources/resource-manager/Microsoft.Authorization/stable/2020-09-01/dataPolicyManifests.json", name: "CloudError"
    ("recoveryservicesbackup", "package-2020-07"), // duplicate fn get_operation_status
    ("network", "package-2017-03-30-only"),  // SchemaNotFound 2017-09-01/network.json SubResource
    ("synapse", "package-2019-06-01-preview"), // TODO #80 path parameters
    ("recoveryservicessiterecovery", "package-2016-08"), // duplicate package-2016-08 https://github.com/Azure/azure-rest-api-specs/pull/11287
    ("mediaservices", "package-2019-05-preview"), // invalid unicode character of a dash instead of a hyphen https://github.com/Azure/azure-rest-api-specs/pull/11576
    // datamigration, same error for all
    // SchemaNotFound MigrateSqlServerSqlDbTask.json ValidationStatus, but may be buried
    ("datamigration", "package-2018-07-15-preview"),
    ("datamigration", "package-2018-04-19"),
    ("datamigration", "package-2018-03-31-preview"),
    ("datamigration", "package-2018-03-15-preview"),
    ("datamigration", "package-2017-11-15-preview"),
];

// becuse of recursive types, some properties have to be boxed
// https://github.com/ctaggart/autorust/issues/73
const BOX_PROPERTIES: &[(&str, &str, &str)] = &[
    // cost-management
    ("../azure-rest-api-specs/specification/cost-management/resource-manager/Microsoft.CostManagement/stable/2020-06-01/costmanagement.json", "ReportConfigFilter", "not"),
    ("../azure-rest-api-specs/specification/cost-management/resource-manager/Microsoft.CostManagement/stable/2020-06-01/costmanagement.json", "QueryFilter", "not"),
    // network
    ("../azure-rest-api-specs/specification/network/resource-manager/Microsoft.Network/stable/2020-07-01/publicIpAddress.json", "PublicIPAddressPropertiesFormat", "ipConfiguration"),
    // databox
    ("../azure-rest-api-specs/specification/databox/resource-manager/Microsoft.DataBox/stable/2020-11-01/databox.json", "transferFilterDetails", "include"),
    ("../azure-rest-api-specs/specification/databox/resource-manager/Microsoft.DataBox/stable/2020-11-01/databox.json", "transferAllDetails", "include"),
    // logic
    ("../azure-rest-api-specs/specification/logic/resource-manager/Microsoft.Logic/stable/2019-05-01/logic.json", "SwaggerSchema", "items"),
    // migrateprojects
    ("../azure-rest-api-specs/specification/migrateprojects/resource-manager/Microsoft.Migrate/preview/2018-09-01-preview/migrate.json", "IEdmNavigationProperty", "partner"),
    ("../azure-rest-api-specs/specification/migrateprojects/resource-manager/Microsoft.Migrate/preview/2018-09-01-preview/migrate.json", "IEdmStructuredType", "baseType"),
    // hardwaresecuritymodels
    ("../azure-rest-api-specs/specification/hardwaresecuritymodules/resource-manager/Microsoft.HardwareSecurityModules/preview/2018-10-31-preview/dedicatedhsm.json", "Error", "innererror"),
];

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("file name was not utf-8"))]
    FileNameNotUtf8Error {},
    IoError {
        source: std::io::Error,
    },
    PathError {
        source: path::Error,
    },
    CodegenError {
        source: autorust_codegen::Error,
    },
    CargoTomlError {
        source: cargo_toml::Error,
    },
    LibRsError {
        source: lib_rs::Error,
    },
}

fn main() -> Result<()> {
    let paths = fs::read_dir(SPEC_FOLDER).context(IoError)?;
    let mut spec_folders = Vec::new();
    for path in paths {
        let path = path.context(IoError)?;
        if path.file_type().context(IoError)?.is_dir() {
            let file_name = path.file_name();
            let spec_folder = file_name.to_str().context(FileNameNotUtf8Error)?;
            spec_folders.push(spec_folder.to_owned());
        }
    }
    spec_folders.sort();

    if ONLY_SERVICES.len() > 0 {
        for (i, spec_folder) in ONLY_SERVICES.iter().enumerate() {
            println!("{} {}", i + 1, spec_folder);
            gen_crate(spec_folder)?;
        }
    } else {
        for (i, spec_folder) in spec_folders.iter().enumerate() {
            println!("{} {}", i + 1, spec_folder);
            if !SKIP_SERVICES.contains(&spec_folder.as_str()) {
                gen_crate(spec_folder)?;
            }
        }
    }
    Ok(())
}

fn gen_crate(spec_folder: &str) -> Result<()> {
    let spec_folder_full = path::join(SPEC_FOLDER, spec_folder).context(PathError)?;
    let readme = &path::join(spec_folder_full, "resource-manager/readme.md").context(PathError)?;
    if !readme.exists() {
        println!("readme not found at {:?}", readme);
        return Ok(());
    }

    let service_name = &get_service_name(spec_folder);
    // println!("{} -> {}", spec_folder, service_name);
    let crate_name = &format!("azure_mgmt_{}", service_name);
    let output_folder = &path::join(OUTPUT_FOLDER, service_name).context(PathError)?;

    let src_folder = path::join(output_folder, "src").context(PathError)?;
    if src_folder.exists() {
        fs::remove_dir_all(&src_folder).context(IoError)?;
    }

    let packages = config_parser::parse_configurations_from_autorest_config_file(&readme);
    let mut feature_mod_names = Vec::new();
    let skip_service_tags: HashSet<&(&str, &str)> = SKIP_SERVICE_TAGS.iter().collect();

    let mut box_properties = HashSet::new();
    for (file_path, schema_name, property_name) in BOX_PROPERTIES {
        box_properties.insert(PropertyName {
            file_path: PathBuf::from(file_path),
            schema_name: schema_name.to_string(),
            property_name: property_name.to_string(),
        });
    }

    for package in packages {
        let tag = package.tag.as_str();
        if let Some(api_version) = to_api_version(&package) {
            if skip_service_tags.contains(&(spec_folder, tag)) {
                // println!("  skipping {}", tag);
                continue;
            }
            // println!("  {}", tag);
            // println!("  {}", api_version);
            let mod_name = &to_mod_name(tag);
            feature_mod_names.push((tag.to_string(), mod_name.clone()));
            // println!("  {}", mod_name);
            let mod_output_folder = path::join(&src_folder, mod_name).context(PathError)?;
            // println!("  {:?}", mod_output_folder);
            // for input_file in &package.input_files {
            //     println!("  {}", input_file);
            // }
            let input_files: Result<Vec<_>> = package
                .input_files
                .iter()
                .map(|input_file| Ok(path::join(readme, input_file).context(PathError)?))
                .collect();
            let input_files = input_files?;
            // for input_file in &input_files {
            //     println!("  {:?}", input_file);
            // }
            autorust_codegen::run(Config {
                api_version: Some(api_version),
                output_folder: mod_output_folder.into(),
                input_files,
                box_properties: box_properties.clone(),
            })
            .context(CodegenError)?;
        }
    }
    if feature_mod_names.len() == 0 {
        return Ok(());
    }
    cargo_toml::create(
        crate_name,
        &feature_mod_names,
        &path::join(output_folder, "Cargo.toml").context(PathError)?,
    )
    .context(CargoTomlError)?;
    lib_rs::create(&feature_mod_names, &path::join(src_folder, "lib.rs").context(PathError)?).context(LibRsError)?;

    Ok(())
}

fn get_service_name(spec_folder: &str) -> String {
    spec_folder.to_snake_case().replace("-", "_")
}
