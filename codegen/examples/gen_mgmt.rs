// cargo run --example gen_mgmt
// https://github.com/Azure/azure-rest-api-specs/blob/master/specification/compute/resource-manager

use autorust_codegen::{
    cargo_toml,
    config_parser::{self, to_api_version, to_mod_name},
    lib_rs, path, run, Config,
};
use heck::SnakeCase;
use indexmap::IndexSet;
use std::{
    collections::{HashMap, HashSet},
    fs,
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

const SPEC_FOLDER: &str = "../azure-rest-api-specs/specification";
const OUTPUT_FOLDER: &str = "../azure-sdk-for-rust/services/mgmt";

const SERVICE_NAMES: &[(&str, &str)] = &[("cosmos-db", "cosmos"), ("vmware", "avs")];

const ONLY_SERVICES: &[&str] = &[
    // "vmware",
    // "resources",
    // "scheduler",
    // "cloudshell",
    // "reservations",
    // "addons",
    ];

const SKIP_SERVICES: &[&str] = &[
    "apimanagement",            // missing properties, all preview apis
    "appconfiguration",         // codegen response wrong, Result<Error> does not serialize
    "appplatform",              // map_type
    "automation",               // Error: Error("data did not match any variant of untagged enum ReferenceOr", line: 90, column: 5)
    "azure_kusto",              // duplicate features in Cargo.toml
    "batch",                    // missing API_VERSION
    "cognitiveservices",        // codegen response wrong, Result<Error> does not serialize
    "containerservice",         // missing generated Expander type
    "cosmos-db",                // get_gremlin_graph_throughput defined twice
    "cost-management",          // use of undeclared crate or module `definition`
    "customproviders",          // properties::ProvisioningState in model not found
    "databox",                  // recursive type has infinite size
    "databoxedge",              // duplicate model pub struct SkuCost {
    "datafactory",
    "datamigration", // Error: "schema not found ../azure-rest-api-specs/specification/datamigration/resource-manager/Microsoft.DataMigration/preview/2018-07-15-preview/definitions/MigrateSqlServerSqlDbTask.json ValidationStatus"
    "deploymentmanager", // missing params
    "deviceprovisioningservices", // missing files
    "dnc",           // conflicting implementation for `v2020_08_08_preview::models::ControllerDetails`
    "hardwaresecuritymodules", // recursive without indirection on Error
    "healthcareapis", // Error: "schema not found ../azure-rest-api-specs/specification/common-types/resource-management/v1/types.json Resource"
    "hybridcompute",  // use of undeclared crate or module `status`
    "intune",         // codegen response wrong, Result<Error> does not serialize
    "keyvault",       // defines Error, recursive type has infinite size
    "logic",          // recursive type has infinite size
    "kubernetesconfiguration", // properties not defined
    "maintenance",    // missing API_VERSION
    "machinelearning", // missing params
    "mariadb",        // Result<Configuration>
    "managedservices", // registration_definition
    "mediaservices",  // Error: Error("invalid unicode code point", line: 1380, column: 289)
    "migrateprojects", // recursive type has infinite size
    "mixedreality",   // &AccountKeyRegenerateRequest not found in scope
    "monitor",        // missing properties
    "mysql",          // Ok200(Configuration)
    "netapp",         // codegen wrong, missing operation params in function
    "network",        // thread 'main' panicked at 'called `Option::unwrap()` on a `None` value', codegen/src/codegen.rs:419:42
    "portal",         // Ok200(Configuration)
    "postgresql",     // Configuration
    "powerplatform", // Error: "parameter not found ../azure-rest-api-specs/specification/powerplatform/resource-manager/Microsoft.PowerPlatform/common/v1/definitions.json ResourceGroupNameParameter"
    "recoveryservicessiterecovery", // duplicate package-2016-08 https://github.com/Azure/azure-rest-api-specs/pull/11287
    "redis",         // map_type
    "relay",         // use of undeclared crate or module `properties`
    "resourcehealth", // undeclared properties
    "search",        // private_link_service_connection_state::Status
    "service-map", // thread 'main' panicked at '"Ref:machine" is not a valid Ident', /Users/cameron/.cargo/registry/src/github.com-1ecc6299db9ec823/proc-macro2-1.0.24/src/fallback.rs:693:9
    "servicebus",  // properties::Action
    "servicefabric", // {}/providers/Microsoft.ServiceFabric/operations list defined twice
    "softwareplan", // Result<Error>
    "storagecache", // use of undeclared crate or module `properties`
    "synapse",     // missing properties
    "web",         // Error: Error("data did not match any variant of untagged enum ReferenceOr", line: 1950, column: 5)
    "windowsesu",  // missing properties
];

const SKIP_SERVICE_TAGS: &[(&str, &str)] = &[
    ("azureactivedirectory", "package-preview-2020-07"),
    ("resources", "package-policy-2020-03"),
    ("recoveryservicesbackup", "package-2020-07"), // duplicate fn get_operation_status
];

fn main() -> Result<()> {
    let paths = fs::read_dir(SPEC_FOLDER).unwrap();
    let mut spec_folders = Vec::new();
    for path in paths {
        let path = path?;
        if path.file_type()?.is_dir() {
            let file_name = path.file_name();
            let spec_folder = file_name.to_str().ok_or("file name")?;
            spec_folders.push(spec_folder.to_owned());
        }
    }
    spec_folders.sort();
    let only_services: IndexSet<&str> = ONLY_SERVICES.iter().cloned().collect();
    if only_services.len() > 0 {
        for (i, spec_folder) in only_services.iter().enumerate() {
            println!("{} {}", i + 1, spec_folder);
            gen_crate(spec_folder)?;
        }
    } else {
        let skip_services: HashSet<&str> = SKIP_SERVICES.iter().cloned().collect();
        for (i, spec_folder) in spec_folders.iter().enumerate() {
            println!("{} {}", i + 1, spec_folder);
            if !skip_services.contains(spec_folder.as_str()) {
                gen_crate(spec_folder)?;
            }
        }
    }
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

fn gen_crate(spec_folder: &str) -> Result<()> {
    let spec_folder_full = path::join(SPEC_FOLDER, spec_folder)?;
    let readme = &path::join(spec_folder_full, "resource-manager/readme.md")?;
    if !readme.exists() {
        println!("not found {:?}", readme);
        return Ok(());
    }

    let service_name = &get_service_name(spec_folder);
    // println!("{} -> {}", spec_folder, service_name);
    let crate_name = &format!("azure_mgmt_{}", service_name);
    let output_folder = &path::join(OUTPUT_FOLDER, service_name)?;
    let src_folder = path::join(output_folder, "src")?;
    if src_folder.exists() {
        fs::remove_dir_all(&src_folder)?;
    }
    // fs::create_dir_all(&src_folder)?;
    let packages = config_parser::parse_configurations_from_autorest_config_file(&readme);
    let mut feature_mod_names: Vec<(String, String)> = Vec::new();
    let skip_service_tags: HashSet<(&str, &str)> = SKIP_SERVICE_TAGS.iter().cloned().collect();
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
            let mod_output_folder = path::join(&src_folder, mod_name)?;
            // println!("  {:?}", mod_output_folder);
            // for input_file in &package.input_files {
            //     println!("  {}", input_file);
            // }
            let input_files: Vec<_> = package
                .input_files
                .iter()
                .map(|input_file| path::join(readme, input_file).unwrap())
                .collect();
            // for input_file in &input_files {
            //     println!("  {:?}", input_file);
            // }
            run(Config {
                api_version: Some(api_version),
                output_folder: mod_output_folder.into(),
                input_files,
            })?;
        }
    }
    if feature_mod_names.len() == 0 {
        return Ok(());
    }
    cargo_toml::create(
        crate_name,
        &feature_mod_names,
        &path::join(output_folder, "Cargo.toml").map_err(|_| "Cargo.toml")?,
    )?;
    lib_rs::create(&feature_mod_names, &path::join(src_folder, "lib.rs").map_err(|_| "lib.rs")?)?;

    Ok(())
}
