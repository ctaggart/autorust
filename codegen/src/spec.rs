use crate::{path, Reference};
use autorust_openapi::{AdditionalProperties, OpenAPI, Operation, Parameter, PathItem, ReferenceOr, Schema};
use heck::SnakeCase;
use indexmap::{IndexMap, IndexSet};
use snafu::{OptionExt, ResultExt, Snafu};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    PathJoin { source: path::Error },
    SchemaNotFound,
    NoNameInReference,
    ParameterNotFound,
    NotImplemented,
    ReadFile { source: std::io::Error },
    DeserializeYaml { source: serde_yaml::Error },
    DeserializeJson { source: serde_json::Error },
}

/// An API specification
#[derive(Clone, Debug)]
pub struct Spec {
    /// A store of all the documents for an API specification keyed on their file paths where the first one is the root document
    pub docs: IndexMap<PathBuf, OpenAPI>,
    schemas: IndexMap<RefKey, Schema>,
    parameters: IndexMap<RefKey, Parameter>,
    input_files_paths: IndexSet<PathBuf>,
}

impl Spec {
    pub fn read_files<P: AsRef<Path>>(input_files_paths: &[P]) -> Result<Self> {
        let mut docs: IndexMap<PathBuf, OpenAPI> = IndexMap::new();
        for input_file in input_files_paths {
            let doc = openapi::parse(&input_file)?;
            let ref_files = openapi::get_ref_files(&doc);
            docs.insert(input_file.as_ref().to_owned(), doc);

            for ref_file in ref_files {
                let doc_path = path::join(&input_file, &ref_file).context(PathJoin)?;
                if !docs.contains_key(&doc_path) {
                    let doc = openapi::parse(&doc_path)?;
                    docs.insert(doc_path, doc);
                }
            }
        }

        let mut schemas: IndexMap<RefKey, Schema> = IndexMap::new();
        let mut parameters: IndexMap<RefKey, Parameter> = IndexMap::new();
        for (path, doc) in &docs {
            for (name, schema) in &doc.definitions {
                match schema {
                    ReferenceOr::Reference { .. } => {}
                    ReferenceOr::Item(schema) => {
                        schemas.insert(
                            RefKey {
                                file: path.clone(),
                                name: name.clone(),
                            },
                            schema.clone(),
                        );
                    }
                }
            }

            for (name, param) in &doc.parameters {
                parameters.insert(
                    RefKey {
                        file: path.clone(),
                        name: name.clone(),
                    },
                    param.clone(),
                );
            }
        }

        Ok(Self {
            docs,
            schemas,
            parameters,
            input_files_paths: input_files_paths.iter().map(|f| f.as_ref().to_owned()).collect(),
        })
    }

    pub fn is_input_file<P: AsRef<Path>>(&self, path: P) -> bool {
        self.input_files_paths.contains(path.as_ref())
    }

    /// Find the schema for a given doc path and reference
    pub fn resolve_schema_ref<P: Into<PathBuf>>(&self, doc_path: P, reference: Reference) -> Result<ResolvedSchema> {
        let doc_path: PathBuf = doc_path.into();
        let full_path = match reference.file {
            None => doc_path,
            Some(file) => path::join(doc_path, &file).context(PathJoin)?,
        };

        let name = reference.name.ok_or_else(|| Error::NoNameInReference)?;
        let ref_key = RefKey { file: full_path, name };
        let schema = self.schemas.get(&ref_key).context(SchemaNotFound)?.clone();
        Ok(ResolvedSchema {
            ref_key: Some(ref_key),
            schema,
        })
    }

    /// Find the parameter for a given doc path and reference
    pub fn resolve_parameter_ref<P: Into<PathBuf>>(&self, doc_path: P, reference: Reference) -> Result<Parameter> {
        let doc_path: PathBuf = doc_path.into();
        let full_path = match reference.file {
            None => doc_path,
            Some(file) => path::join(doc_path, &file).context(PathJoin)?,
        };
        let name = reference.name.ok_or_else(|| Error::NoNameInReference)?;
        Ok(self
            .parameters
            .get(&RefKey { file: full_path, name })
            .context(ParameterNotFound)?
            .clone())
    }

    /// Resolve a reference or schema to a resolved schema
    pub fn resolve_schema<P: AsRef<Path>>(&self, doc_path: P, ref_or_schema: &ReferenceOr<Schema>) -> Result<ResolvedSchema> {
        match ref_or_schema {
            ReferenceOr::Item(schema) => Ok(ResolvedSchema {
                ref_key: None,
                schema: schema.clone(),
            }),
            ReferenceOr::Reference { reference, .. } => self.resolve_schema_ref(doc_path.as_ref(), Reference::parse(reference)),
        }
    }

    /// Resolve a collection of references or schemas to a collection of resolved schemas
    pub fn resolve_schemas<P: AsRef<Path>>(&self, doc_path: P, ref_or_schemas: &[ReferenceOr<Schema>]) -> Result<Vec<ResolvedSchema>> {
        let mut resolved = Vec::new();
        for schema in ref_or_schemas {
            resolved.push(self.resolve_schema(&doc_path, schema)?);
        }
        Ok(resolved)
    }

    /// Resolve a collection of references or schemas to a collection of resolved schemas
    pub fn resolve_schema_map<P: AsRef<Path>>(
        &self,
        doc_path: P,
        ref_or_schemas: &IndexMap<String, ReferenceOr<Schema>>,
    ) -> Result<IndexMap<String, ResolvedSchema>> {
        let mut resolved = IndexMap::new();
        for (name, schema) in ref_or_schemas {
            resolved.insert(name.clone(), self.resolve_schema(&doc_path, schema)?);
        }
        Ok(resolved)
    }

    pub fn resolve_path<P: AsRef<Path>>(&self, _doc_path: P, path: &ReferenceOr<PathItem>) -> Result<PathItem> {
        match path {
            ReferenceOr::Item(path) => Ok(path.clone()),
            ReferenceOr::Reference { .. } =>
            // self.resolve_path_ref(doc_file, reference),
            {
                // TODO
                NotImplemented.fail()
            }
        }
    }

    pub fn resolve_path_map(&self, doc_file: &Path, paths: &IndexMap<String, ReferenceOr<PathItem>>) -> Result<IndexMap<String, PathItem>> {
        let mut resolved = IndexMap::new();
        for (name, path) in paths {
            resolved.insert(name.clone(), self.resolve_path(doc_file, path)?);
        }
        Ok(resolved)
    }

    pub fn resolve_parameter(&self, doc_file: &Path, parameter: &ReferenceOr<Parameter>) -> Result<Parameter> {
        match parameter {
            ReferenceOr::Item(param) => Ok(param.clone()),
            ReferenceOr::Reference { reference, .. } => self.resolve_parameter_ref(doc_file, Reference::parse(reference)),
        }
    }

    pub fn resolve_parameters(&self, doc_file: &Path, parameters: &Vec<ReferenceOr<Parameter>>) -> Result<Vec<Parameter>> {
        let mut resolved = Vec::new();
        for param in parameters {
            resolved.push(self.resolve_parameter(doc_file, param)?);
        }
        Ok(resolved)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RefKey {
    pub file: PathBuf,
    pub name: String,
}

pub struct ResolvedSchema {
    pub ref_key: Option<RefKey>,
    pub schema: Schema,
}

pub mod openapi {
    use super::*;

    /// Parse an OpenAPI object from a file located at `path`
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<OpenAPI> {
        let path = path.as_ref();
        let bytes = fs::read(path).context(ReadFile)?;
        let api = if path.extension() == Some(OsStr::new("yaml")) || path.extension() == Some(OsStr::new("yml")) {
            serde_yaml::from_slice(&bytes).context(DeserializeYaml)?
        } else {
            serde_json::from_slice(&bytes).context(DeserializeJson)?
        };

        Ok(api)
    }

    /// Returns a set of referenced relative file paths
    pub fn get_ref_files(api: &OpenAPI) -> IndexSet<String> {
        get_refs(api)
            .iter()
            .filter_map(|rf| match rf {
                RefString::Example(_) => None,
                rs => Reference::parse(rs.as_str()).file,
            })
            .collect()
    }
}

pub enum OperationVerb<'a> {
    Get(&'a Operation),
    Post(&'a Operation),
    Put(&'a Operation),
    Patch(&'a Operation),
    Delete(&'a Operation),
    Options(&'a Operation),
    Head(&'a Operation),
}

// Hold an operation and remembers the operation verb.
impl<'a> OperationVerb<'a> {
    pub fn operation(&self) -> &'a Operation {
        match self {
            OperationVerb::Get(op) => op,
            OperationVerb::Post(op) => op,
            OperationVerb::Put(op) => op,
            OperationVerb::Patch(op) => op,
            OperationVerb::Delete(op) => op,
            OperationVerb::Options(op) => op,
            OperationVerb::Head(op) => op,
        }
    }

    pub fn verb_name(&self) -> &'static str {
        match self {
            OperationVerb::Get(_) => "get",
            OperationVerb::Post(_) => "post",
            OperationVerb::Put(_) => "put",
            OperationVerb::Patch(_) => "patch",
            OperationVerb::Delete(_) => "delete",
            OperationVerb::Options(_) => "options",
            OperationVerb::Head(_) => "head",
        }
    }

    pub fn function_name(&self, path: &str) -> (Option<String>, String) {
        if let Some(operation_id) = &self.operation().operation_id {
            function_name_from_operation_id(operation_id)
        } else {
            (None, create_function_name(path, self.verb_name()))
        }
    }
}

/// Returns the module name and function name.
/// The module name is optional and is text before an underscore in the operatonId.
fn function_name_from_operation_id(operation_id: &str) -> (Option<String>, String) {
    let parts: Vec<&str> = operation_id.splitn(2, '_').collect();
    if parts.len() == 2 {
        (Some(parts[0].to_snake_case()), parts[1].to_snake_case())
    } else {
        (None, parts[0].to_snake_case())
    }
}

/// Creating a function name from the path and verb when an operationId is not specified.
/// All azure-rest-api-specs operations should have an operationId.
fn create_function_name(path: &str, verb_name: &str) -> String {
    let mut path = path.split('/').filter(|&x| !x.is_empty()).collect::<Vec<_>>();
    path.push(verb_name);
    path.join("_")
}

pub fn path_item_operations(item: &PathItem) -> impl Iterator<Item = OperationVerb> {
    vec![
        item.get.as_ref().map(OperationVerb::Get),
        item.post.as_ref().map(OperationVerb::Post),
        item.put.as_ref().map(OperationVerb::Put),
        item.patch.as_ref().map(OperationVerb::Patch),
        item.delete.as_ref().map(OperationVerb::Delete),
        item.options.as_ref().map(OperationVerb::Options),
        item.head.as_ref().map(OperationVerb::Head),
    ]
    .into_iter()
    .filter_map(|x| x)
}

/// Holds a $ref string and remembers the type.
#[derive(Clone, Debug, PartialEq)]
pub enum RefString {
    PathItem(String),
    Parameter(String),
    Schema(String),
    Example(String),
}

impl RefString {
    fn as_str(&self) -> &str {
        match self {
            RefString::PathItem(s) => s,
            RefString::Parameter(s) => s,
            RefString::Schema(s) => s,
            RefString::Example(s) => s,
        }
    }
}

impl ToString for RefString {
    fn to_string(&self) -> String {
        self.as_str().to_owned()
    }
}

fn add_refs_for_schema(list: &mut Vec<RefString>, schema: &Schema) {
    for (_, schema) in &schema.properties {
        match schema {
            ReferenceOr::Reference { reference, .. } => list.push(RefString::Schema(reference.to_owned())),
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        }
    }
    match schema.additional_properties.as_ref() {
        Some(ap) => match ap {
            AdditionalProperties::Boolean(_) => {}
            AdditionalProperties::Schema(schema) => match schema {
                ReferenceOr::Reference { reference, .. } => list.push(RefString::Schema(reference.to_owned())),
                ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
            },
        },
        _ => {}
    }
    match schema.common.items.as_ref() {
        Some(schema) => match schema {
            ReferenceOr::Reference { reference, .. } => list.push(RefString::Schema(reference.to_owned())),
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        },
        _ => {}
    }
    for schema in &schema.all_of {
        match schema {
            ReferenceOr::Reference { reference, .. } => list.push(RefString::Schema(reference.to_owned())),
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        }
    }
}

/// Returns the list of all refs for an OpenAPI schema
pub fn get_refs(api: &OpenAPI) -> Vec<RefString> {
    let mut list = Vec::new();

    // paths and operations
    for (_path, item) in &api.paths {
        match item {
            ReferenceOr::Reference { reference, .. } => list.push(RefString::PathItem(reference.clone())),
            ReferenceOr::Item(item) => {
                for verb in path_item_operations(&item) {
                    let op = verb.operation();
                    // parameters
                    for param in &op.parameters {
                        match param {
                            ReferenceOr::Reference { reference, .. } => list.push(RefString::Parameter(reference.clone())),
                            ReferenceOr::Item(parameter) => match &parameter.schema {
                                Some(ReferenceOr::Reference { reference, .. }) => list.push(RefString::Schema(reference.clone())),
                                Some(ReferenceOr::Item(schema)) => add_refs_for_schema(&mut list, schema),
                                None => {}
                            },
                        }
                    }

                    // responses
                    for (_code, rsp) in &op.responses {
                        match &rsp.schema {
                            Some(ReferenceOr::Reference { reference, .. }) => list.push(RefString::Schema(reference.to_owned())),
                            Some(ReferenceOr::Item(schema)) => add_refs_for_schema(&mut list, schema),
                            None => {}
                        }
                    }

                    // examples
                    for (_name, example) in &op.x_ms_examples {
                        match example {
                            ReferenceOr::Reference { reference, .. } => list.push(RefString::Example(reference.to_owned())),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // definitions
    for (_name, schema) in &api.definitions {
        match schema {
            ReferenceOr::Reference { reference, .. } => list.push(RefString::Schema(reference.to_owned())),
            ReferenceOr::Item(schema) => add_refs_for_schema(&mut list, schema),
        }
    }

    list
}

pub fn get_api_schema_refs(api: &OpenAPI) -> Vec<String> {
    get_refs(api)
        .iter()
        .filter_map(|rf| match rf {
            RefString::Schema(rs) => Some(rs.to_owned()),
            _ => None,
        })
        .collect()
}

pub fn get_schema_schema_refs(schema: &Schema) -> Vec<String> {
    let mut refs = Vec::new();
    add_refs_for_schema(&mut refs, schema);
    refs.iter()
        .filter_map(|rf| match rf {
            RefString::Schema(rs) => Some(rs.to_owned()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_function_name() {
        assert_eq!(create_function_name("/pets", "get"), "pets_get");
    }

    #[test]
    fn test_function_name_from_operation_id() {
        assert_eq!(
            function_name_from_operation_id("PrivateClouds_CreateOrUpdate"),
            (Some("private_clouds".to_owned()), "create_or_update".to_owned())
        );
        assert_eq!(function_name_from_operation_id("get"), (None, "get".to_owned()));
    }
}
