pub mod cargo_toml;
mod codegen;
pub mod config_parser;
pub mod identifier;
pub mod lib_rs;
pub mod path;
pub mod spec;
mod status_codes;

pub use self::{
    codegen::{create_mod, CodeGen},
    spec::{OperationVerb, ResolvedSchema, Spec},
};

use config_parser::Configuration;
use proc_macro2::TokenStream;
use snafu::{OptionExt, ResultExt, Snafu};

use std::{
    collections::HashSet,
    fs::{self, File},
    io::prelude::*,
    path::{Path, PathBuf},
};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not create output directory {}: {}", directory.display(), source))]
    CreateOutputDirectoryError {
        directory: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not create file {}: {}", file.display(), source))]
    CreateFileError {
        file: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not write file {}: {}", file.display(), source))]
    WriteFileError {
        file: PathBuf,
        source: std::io::Error,
    },
    CodeGenNewError {
        source: codegen::Error,
    },
    #[snafu(display("CreateModelsError {} {}", config.output_folder.display(), source))]
    CreateModelsError {
        source: codegen::Error,
        config: Config,
    },
    CreateOperationsError {
        source: codegen::Error,
    },
    PathError {
        source: path::Error,
    },
    IoError {
        source: std::io::Error,
    },
    #[snafu(display("file name was not utf-8"))]
    FileNameNotUtf8Error {},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PropertyName {
    pub file_path: PathBuf,
    pub schema_name: String,
    pub property_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub input_files: Vec<PathBuf>,
    pub output_folder: PathBuf,
    pub api_version: Option<String>,
    pub box_properties: HashSet<PropertyName>,
}

pub fn run(config: Config) -> Result<()> {
    let directory = &config.output_folder;
    fs::create_dir_all(directory).context(CreateOutputDirectoryError { directory })?;
    let cg = &CodeGen::new(config.clone()).context(CodeGenNewError)?;

    // create models from schemas
    let models = cg.create_models().context(CreateModelsError { config: config.clone() })?;
    let models_path = path::join(&config.output_folder, "models.rs").context(PathError)?;
    write_file(&models_path, &models)?;

    // create api client from operations
    let operations = cg.create_operations().context(CreateOperationsError)?;
    let operations_path = path::join(&config.output_folder, "operations.rs").context(PathError)?;
    write_file(&operations_path, &operations)?;

    if let Some(api_version) = &config.api_version {
        let operations = create_mod(api_version);
        let operations_path = path::join(&config.output_folder, "mod.rs").context(PathError)?;
        write_file(&operations_path, &operations)?;
    }
    Ok(())
}

fn write_file<P: AsRef<Path>>(file: P, tokens: &TokenStream) -> Result<()> {
    let file = file.as_ref();
    // println!("writing file {}", &file.display());
    let code = tokens.to_string();
    let mut buffer = File::create(&file).context(CreateFileError { file: file.clone() })?;
    buffer.write_all(&code.as_bytes()).context(WriteFileError { file })?;
    Ok(())
}

const SPEC_FOLDER: &str = "../azure-rest-api-specs/specification";

// gets a sorted list of folders in ../azure-rest-api-specs/specification
fn get_spec_folders(spec_folder: &str) -> Result<Vec<String>, Error> {
    let paths = fs::read_dir(spec_folder).context(IoError)?;
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
    Ok(spec_folders)
}

const RESOURCE_MANAGER_README: &str = "resource-manager/readme.md";
const DATA_PLANE_README: &str = "data-plane/readme.md";

pub fn get_mgmt_configs() -> Result<Vec<SpecConfigs>> {
    get_spec_configs(SPEC_FOLDER, &RESOURCE_MANAGER_README)
}

pub fn get_svc_configs() -> Result<Vec<SpecConfigs>> {
    get_spec_configs(SPEC_FOLDER, &DATA_PLANE_README)
}

fn get_readme(spec_folder_full: &dyn AsRef<Path>, readme_kind: &dyn AsRef<Path>) -> Option<PathBuf> {
    match path::join(spec_folder_full, readme_kind) {
        Ok(readme) => {
            if readme.exists() {
                Some(readme)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

pub struct SpecConfigs {
    spec: String,
    readme: PathBuf,
    configs: Vec<Configuration>,
}

impl SpecConfigs {
    pub fn spec(&self) -> &str {
        self.spec.as_str()
    }
    pub fn readme(&self) -> &Path {
        self.readme.as_path()
    }
    pub fn configs(&self) -> &Vec<Configuration> {
        self.configs.as_ref()
    }
}

fn get_spec_configs(spec_folder: &str, readme_kind: &dyn AsRef<Path>) -> Result<Vec<SpecConfigs>> {
    let specs = get_spec_folders(spec_folder)?;
    Ok(specs
        .into_iter()
        .filter_map(|spec| match path::join(SPEC_FOLDER, &spec) {
            Ok(spec_folder_full) => match get_readme(&spec_folder_full, readme_kind) {
                Some(readme) => {
                    let configs = config_parser::parse_configurations_from_autorest_config_file(&readme);
                    Some(SpecConfigs { spec, readme, configs })
                }
                None => None,
            },
            Err(_) => None,
        })
        .collect())
}
