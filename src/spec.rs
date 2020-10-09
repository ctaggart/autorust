use crate::{path, Reference, Result};
use autorust_openapi::{
    AdditionalProperties, OpenAPI, Operation, Parameter, PathItem, ReferenceOr, Schema,
};
use indexmap::{IndexMap, IndexSet};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

/// An API specification
#[derive(Clone, Debug)]
pub struct Spec {
    /// Documents for an API specification where the first one is the root
    pub docs: IndexMap<PathBuf, OpenAPI>,
    schemas: IndexMap<RefKey, Schema>,
    parameters: IndexMap<RefKey, Parameter>,
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

impl Spec {
    pub fn root(&self) -> (&Path, &OpenAPI) {
        let (file, doc) = self.docs.get_index(0).unwrap();
        (file, doc)
    }

    pub fn root_file(&self) -> &Path {
        let (file, _doc) = self.root();
        file
    }

    pub fn root_doc(&self) -> &OpenAPI {
        let (_file, doc) = self.root();
        doc
    }

    pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_owned();

        let mut docs = IndexMap::new();
        let root = read_api_file(&path)?;
        let files = get_ref_files(&root)?;
        docs.insert(path.clone(), root);

        for file in files {
            let doc_path = path::join(&path, &file)?;
            let doc = read_api_file(&doc_path)?;
            docs.insert(doc_path, doc);
        }

        let mut schemas: IndexMap<RefKey, Schema> = IndexMap::new();
        let mut parameters: IndexMap<RefKey, Parameter> = IndexMap::new();
        for (file, doc) in &docs {
            for (name, schema) in &doc.definitions {
                match schema {
                    ReferenceOr::Reference { .. } => {}
                    ReferenceOr::Item(schema) => {
                        // println!("insert schema {} {}", &file, &name);
                        schemas.insert(
                            RefKey {
                                file: file.clone(),
                                name: name.clone(),
                            },
                            schema.clone(),
                        );
                    }
                }
            }
            for (name, param) in &doc.parameters {
                // println!("insert parameter {} {}", &file, name);
                parameters.insert(
                    RefKey {
                        file: file.clone(),
                        name: name.clone(),
                    },
                    param.clone(),
                );
            }
        }

        Ok(Spec {
            docs,
            schemas,
            parameters,
        })
    }

    pub fn resolve_schema_ref(&self, doc_file: &Path, reference: &str) -> Result<ResolvedSchema> {
        let rf = Reference::parse(reference)?;
        let file = match rf.file {
            None => doc_file.to_owned(),
            Some(file) => path::join(doc_file, &file)?,
        };
        match rf.name {
            None => Err(format!("no name in reference {}", &reference))?,
            Some(nm) => {
                let ref_key = RefKey {
                    file: file.clone(),
                    name: nm.clone(),
                };
                let schema = self
                    .schemas
                    .get(&ref_key)
                    .ok_or_else(|| format!("schema not found {} {}", &file.display(), &nm))?
                    .clone();
                Ok(ResolvedSchema {
                    ref_key: Some(ref_key),
                    schema,
                })
            }
        }
    }

    pub fn resolve_parameter_ref(&self, doc_file: &Path, reference: &str) -> Result<Parameter> {
        let rf = Reference::parse(reference)?;
        let file = match rf.file {
            None => doc_file.to_owned(),
            Some(file) => path::join(doc_file, &file)?,
        };
        match rf.name {
            None => Err(format!("no name in reference {}", &reference))?,
            Some(nm) => Ok(self
                .parameters
                .get(&RefKey {
                    file: file.clone(),
                    name: nm.clone(),
                })
                .ok_or_else(|| format!("parameter not found {} {}", &file.display(), &nm))?
                .clone()),
        }
    }

    pub fn resolve_schema(
        &self,
        doc_file: &Path,
        schema: &ReferenceOr<Schema>,
    ) -> Result<ResolvedSchema> {
        match schema {
            ReferenceOr::Item(schema) => Ok(ResolvedSchema {
                ref_key: None,
                schema: schema.clone(),
            }),
            ReferenceOr::Reference { reference, .. } => {
                self.resolve_schema_ref(doc_file, reference)
            }
        }
    }

    pub fn resolve_schema_map(
        &self,
        doc_file: &Path,
        schemas: &IndexMap<String, ReferenceOr<Schema>>,
    ) -> Result<IndexMap<String, ResolvedSchema>> {
        let mut resolved = IndexMap::new();
        for (name, schema) in schemas {
            resolved.insert(name.clone(), self.resolve_schema(doc_file, schema)?);
        }
        Ok(resolved)
    }

    pub fn resolve_path(&self, _doc_file: &Path, path: &ReferenceOr<PathItem>) -> Result<PathItem> {
        match path {
            ReferenceOr::Item(path) => Ok(path.clone()),
            ReferenceOr::Reference { .. } =>
            // self.resolve_path_ref(doc_file, reference),
            {
                Err("path references not implemented")?
            } // TODO
        }
    }

    pub fn resolve_path_map(
        &self,
        doc_file: &Path,
        paths: &IndexMap<String, ReferenceOr<PathItem>>,
    ) -> Result<IndexMap<String, PathItem>> {
        let mut resolved = IndexMap::new();
        for (name, path) in paths {
            resolved.insert(name.clone(), self.resolve_path(doc_file, path)?);
        }
        Ok(resolved)
    }

    pub fn resolve_parameter(
        &self,
        doc_file: &Path,
        parameter: &ReferenceOr<Parameter>,
    ) -> Result<Parameter> {
        match parameter {
            ReferenceOr::Item(param) => Ok(param.clone()),
            ReferenceOr::Reference { reference, .. } => {
                self.resolve_parameter_ref(doc_file, reference)
            }
        }
    }

    pub fn resolve_parameters(
        &self,
        doc_file: &Path,
        parameters: &Vec<ReferenceOr<Parameter>>,
    ) -> Result<Vec<Parameter>> {
        let mut resolved = Vec::new();
        for param in parameters {
            resolved.push(self.resolve_parameter(doc_file, param)?);
        }
        Ok(resolved)
    }
}

pub fn read_api_file<P: AsRef<Path>>(path: P) -> Result<OpenAPI> {
    let path = path.as_ref();
    let bytes = fs::read(path)?;
    let api = if path.extension() == Some(OsStr::new("yaml"))
        || path.extension() == Some(OsStr::new("yml"))
    {
        serde_yaml::from_slice(&bytes)?
    } else {
        serde_json::from_slice(&bytes)?
    };

    Ok(api)
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
}

pub fn pathitem_operations(item: &PathItem) -> impl Iterator<Item = OperationVerb> {
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

impl ToString for RefString {
    fn to_string(&self) -> String {
        match self {
            RefString::PathItem(s) => s.to_owned(),
            RefString::Parameter(s) => s.to_owned(),
            RefString::Schema(s) => s.to_owned(),
            RefString::Example(s) => s.to_owned(),
        }
    }
}

fn add_refs_for_schema(list: &mut Vec<RefString>, schema: &Schema) {
    for (_, schema) in &schema.properties {
        match schema {
            ReferenceOr::Reference { reference, .. } => {
                list.push(RefString::Schema(reference.to_owned()))
            }
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        }
    }
    match schema.additional_properties.as_ref() {
        Some(ap) => match ap {
            AdditionalProperties::Boolean(_) => {}
            AdditionalProperties::Schema(schema) => match schema {
                ReferenceOr::Reference { reference, .. } => {
                    list.push(RefString::Schema(reference.to_owned()))
                }
                ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
            },
        },
        _ => {}
    }
    match schema.common.items.as_ref() {
        Some(schema) => match schema {
            ReferenceOr::Reference { reference, .. } => {
                list.push(RefString::Schema(reference.to_owned()))
            }
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        },
        _ => {}
    }
    for schema in &schema.all_of {
        match schema {
            ReferenceOr::Reference { reference, .. } => {
                list.push(RefString::Schema(reference.to_owned()))
            }
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
            ReferenceOr::Reference { reference, .. } => {
                list.push(RefString::PathItem(reference.clone()))
            }
            ReferenceOr::Item(item) => {
                for verb in pathitem_operations(&item) {
                    let op = verb.operation();
                    // parameters
                    for prm in &op.parameters {
                        match prm {
                            ReferenceOr::Reference { reference, .. } => {
                                list.push(RefString::Parameter(reference.clone()))
                            }
                            ReferenceOr::Item(parameter) => match &parameter.schema {
                                Some(ReferenceOr::Reference { reference, .. }) => {
                                    list.push(RefString::Schema(reference.clone()))
                                }
                                Some(ReferenceOr::Item(schema)) => {
                                    add_refs_for_schema(&mut list, schema)
                                }
                                None => {}
                            },
                        }
                    }

                    // responses
                    for (_code, rsp) in &op.responses {
                        match &rsp.schema {
                            Some(ReferenceOr::Reference { reference, .. }) => {
                                list.push(RefString::Schema(reference.to_owned()))
                            }
                            Some(ReferenceOr::Item(schema)) => {
                                add_refs_for_schema(&mut list, schema)
                            }
                            None => {}
                        }
                    }

                    // examples
                    for (_name, example) in &op.x_ms_examples {
                        match example {
                            ReferenceOr::Reference { reference, .. } => {
                                list.push(RefString::Example(reference.to_owned()))
                            }
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
            ReferenceOr::Reference { reference, .. } => {
                list.push(RefString::Schema(reference.to_owned()))
            }
            ReferenceOr::Item(schema) => add_refs_for_schema(&mut list, schema),
        }
    }

    list
}

/// returns a set of referenced files
pub fn get_ref_files(api: &OpenAPI) -> Result<IndexSet<String>> {
    let ref_strings: IndexSet<_> = get_refs(api)
        .iter()
        .filter_map(|rf| match rf {
            RefString::Example(_) => None,
            rs => Some(rs.to_string()),
        })
        .collect();

    let mut set = IndexSet::new();
    for s in &ref_strings {
        if let Some(file) = Reference::parse(s)?.file {
            set.insert(file);
        }
    }

    Ok(set)
}
