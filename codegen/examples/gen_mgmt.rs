// cargo run --example gen_mgmt
// https://github.com/Azure/azure-rest-api-specs/blob/master/specification/compute/resource-manager
use autorust_codegen::{
    self, cargo_toml,
    config_parser::{self, to_api_version, to_mod_name},
    lib_rs, path, Config,
};
use heck::SnakeCase;
use snafu::{OptionExt, ResultExt, Snafu};
use std::{
    collections::{HashMap, HashSet},
    fs,
};

const SPEC_FOLDER: &str = "../azure-rest-api-specs/specification";
const OUTPUT_FOLDER: &str = "../azure-sdk-for-rust/services/mgmt";

const SERVICE_NAMES: &[(&str, &str)] = &[
    // ("cosmos-db", "cosmos"),
    // ("vmware", "avs")
    ];

const ONLY_SERVICES: &[&str] = &[
    // "network",
    // "redis",
];

const SKIP_SERVICES: &[&str] = &[
    "apimanagement",                // missing properties, all preview apis
    "automation",                   // 'not yet implemented: Handle DataType::File
    "cosmos-db",                    // get_gremlin_graph_throughput defined twice
    "cost-management",              // use of undeclared crate or module `definition`
    "databox",                      // TODO #73 recursive types
    "databoxedge",                  // duplicate model pub struct SkuCost {
    "datamigration", // Error: "schema not found ../azure-rest-api-specs/specification/datamigration/resource-manager/Microsoft.DataMigration/preview/2018-07-15-preview/definitions/MigrateSqlServerSqlDbTask.json ValidationStatus"
    "deploymentmanager", // missing params
    "deviceprovisioningservices", // certificate_name used as parameter more than once
    "dnc",           // conflicting implementation for `v2020_08_08_preview::models::ControllerDetails`
    "hardwaresecuritymodules", // recursive without indirection on Error
    "healthcareapis", // Error: "schema not found ../azure-rest-api-specs/specification/common-types/resource-management/v1/types.json Resource"
    "logic",          // TODO #73 recursive types
    "machinelearning", // missing params
    "mediaservices",  // Error: Error("invalid unicode code point", line: 1380, column: 289)
    "migrateprojects", // TODO #73 recursive types
    "mixedreality",   // &AccountKeyRegenerateRequest not found in scope
    "netapp",         // codegen wrong, missing operation params in function
    "network",        // TODO #73 recursive types
    "powerplatform", // Error: "parameter not found ../azure-rest-api-specs/specification/powerplatform/resource-manager/Microsoft.PowerPlatform/common/v1/definitions.json ResourceGroupNameParameter"
    "recoveryservicessiterecovery", // duplicate package-2016-08 https://github.com/Azure/azure-rest-api-specs/pull/11287
    "redis", // SchemaNotFound { ref_key: RefKey { file_path: "../azure-rest-api-specs/specification/common-types/resource-management/v1/types.json", name: "Resource"
    "redisenterprise", // SchemaNotFound { ref_key: RefKey { file_path: "../azure-rest-api-specs/specification/common-types/resource-management/v1/types.json", name: "Resource"
    "service-map", // thread 'main' panicked at '"Ref:machine" is not a valid Ident', /Users/cameron/.cargo/registry/src/github.com-1ecc6299db9ec823/proc-macro2-1.0.24/src/fallback.rs:693:9
    "servicefabric", // {}/providers/Microsoft.ServiceFabric/operations list defined twice
    "web",         // Error: Error("data did not match any variant of untagged enum ReferenceOr", line: 1950, column: 5)
];

const SKIP_SERVICE_TAGS: &[(&str, &str)] = &[
    ("azureactivedirectory", "package-preview-2020-07"),
    ("resources", "package-policy-2020-03"),
    ("resources", "package-policy-2020-09"), // SchemaNotFound { ref_key: RefKey { file_path: "../azure-rest-api-specs/specification/resources/resource-manager/Microsoft.Authorization/stable/2020-09-01/dataPolicyManifests.json", name: "CloudError"
    ("recoveryservicesbackup", "package-2020-07"), // duplicate fn get_operation_status
    ("network", "package-2017-03-30-only"),  // SchemaNotFound 2017-09-01/network.json SubResource
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
    let service_names: HashMap<_, _> = SERVICE_NAMES.iter().cloned().collect();
    if let Some(service_name) = service_names.get(spec_folder) {
        service_name.to_string()
    } else {
        spec_folder.to_snake_case().replace("-", "_")
    }
}
