use crate::{path_join, Reference, Result};
use autorust_openapi::{OpenAPI, Operation, Parameter, PathItem, ReferenceOr, Schema};
use indexmap::{IndexMap, IndexSet};
use std::{fs::File, io::prelude::*};

#[derive(Clone, Debug)]
pub struct Spec {
    /// multiple documents for an API specification
    /// the first one is the root
    pub docs: IndexMap<String, OpenAPI>,
    schemas: IndexMap<RefKey, Schema>,
    parameters: IndexMap<RefKey, Parameter>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RefKey {
    file: String,
    name: String,
}

impl Spec {
    pub fn root(&self) -> (&str, &OpenAPI) {
        let (file, doc) = self.docs.get_index(0).unwrap();
        (file, doc)
    }
    pub fn root_file(&self) -> &str {
        let (file, _doc) = self.root();
        file
    }
    pub fn root_doc(&self) -> &OpenAPI {
        let (_file, doc) = self.root();
        doc
    }

    pub fn read_file(path: &str) -> Result<Self> {
        let mut docs = IndexMap::new();
        let root = read_api_file(path)?;
        let files = get_ref_files(&root)?;
        docs.insert(path.to_owned(), root);
        for file in files {
            let doc_path = path_join(true, path, &file)?;
            let doc = read_api_file(&doc_path)?;
            docs.insert(doc_path, doc);
        }

        let mut schemas: IndexMap<RefKey, Schema> = IndexMap::new();
        let mut parameters: IndexMap<RefKey, Parameter> = IndexMap::new();
        for (file, doc) in &docs {
            for (name, schema) in &doc.definitions {
                match schema {
                    ReferenceOr::Reference { reference: _ } => {}
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
                // println!("insert parameter {} {}", &file, &name);
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

    pub fn resolve_schema_ref(&self, doc_file: &str, reference: &str) -> Result<Schema> {
        let rf = Reference::parse(reference)?;
        let file = match rf.file {
            None => doc_file.to_owned(),
            Some(file) => path_join(true, doc_file, &file)?,
        };
        match rf.name {
            None => Err(format!("no name in reference {}", &reference))?,
            Some(nm) => Ok(self
                .schemas
                .get(&RefKey {
                    file: file.clone(),
                    name: nm.clone(),
                })
                .ok_or_else(|| format!("schema not found {} {}", &file, &nm))?
                .clone()),
        }
    }

    pub fn resolve_parameter_ref(&self, doc_file: &str, reference: &str) -> Result<Parameter> {
        let rf = Reference::parse(reference)?;
        let file = match rf.file {
            None => doc_file.to_owned(),
            Some(file) => path_join(true, doc_file, &file)?,
        };
        match rf.name {
            None => Err(format!("no name in reference {}", &reference))?,
            Some(nm) => Ok(self
                .parameters
                .get(&RefKey {
                    file: file.clone(),
                    name: nm.clone(),
                })
                .ok_or_else(|| format!("parameter not found {} {}", &file, &nm))?
                .clone()),
        }
    }

    pub fn resolve_schema(&self, doc_file: &str, schema: &ReferenceOr<Schema>) -> Result<Schema> {
        match schema {
            ReferenceOr::Item(schema) => Ok(schema.clone()),
            ReferenceOr::Reference { reference } => self.resolve_schema_ref(doc_file, reference),
        }
    }

    pub fn resolve_schema_map(
        &self,
        doc_file: &str,
        schemas: &IndexMap<String, ReferenceOr<Schema>>,
    ) -> Result<IndexMap<String, Schema>> {
        let mut resolved = IndexMap::new();
        for (name, schema) in schemas {
            resolved.insert(name.clone(), self.resolve_schema(doc_file, schema)?);
        }
        Ok(resolved)
    }

    pub fn resolve_path(&self, _doc_file: &str, path: &ReferenceOr<PathItem>) -> Result<PathItem> {
        match path {
            ReferenceOr::Item(path) => Ok(path.clone()),
            ReferenceOr::Reference { reference: _ } =>
            // self.resolve_path_ref(doc_file, reference),
            {
                Err("path references not implemented")?
            } // TODO
        }
    }

    pub fn resolve_path_map(
        &self,
        doc_file: &str,
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
        doc_file: &str,
        parameter: &ReferenceOr<Parameter>,
    ) -> Result<Parameter> {
        match parameter {
            ReferenceOr::Item(param) => Ok(param.clone()),
            ReferenceOr::Reference { reference } => self.resolve_parameter_ref(doc_file, reference),
        }
    }

    pub fn resolve_parameters(
        &self,
        doc_file: &str,
        parameters: &Vec<ReferenceOr<Parameter>>,
    ) -> Result<Vec<Parameter>> {
        let mut resolved = Vec::new();
        for param in parameters {
            resolved.push(self.resolve_parameter(doc_file, param)?);
        }
        Ok(resolved)
    }
}

pub fn read_api_file(path: &str) -> Result<OpenAPI> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    let api = serde_json::from_slice(&bytes)?;
    Ok(api)
}

pub fn pathitem_operations(item: &PathItem) -> impl Iterator<Item = &Operation> {
    vec![
        item.get.as_ref(),
        item.post.as_ref(),
        item.put.as_ref(),
        item.patch.as_ref(),
        item.delete.as_ref(),
        item.options.as_ref(),
        item.head.as_ref(),
    ]
    .into_iter()
    .filter_map(|x| x)
}

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
            ReferenceOr::Reference { reference } => {
                list.push(RefString::Schema(reference.to_owned()))
            }
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        }
    }
    match schema.additional_properties.as_ref() {
        Some(schema) => match schema {
            ReferenceOr::Reference { reference } => {
                list.push(RefString::Schema(reference.to_owned()))
            }
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        },
        _ => {}
    }
    match schema.items.as_ref() {
        Some(schema) => match schema {
            ReferenceOr::Reference { reference } => {
                list.push(RefString::Schema(reference.to_owned()))
            }
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        },
        _ => {}
    }
    for schema in &schema.all_of {
        match schema {
            ReferenceOr::Reference { reference } => {
                list.push(RefString::Schema(reference.to_owned()))
            }
            ReferenceOr::Item(schema) => add_refs_for_schema(list, schema),
        }
    }
}

/// returns a list of refs
pub fn get_refs(api: &OpenAPI) -> Vec<RefString> {
    let mut list = Vec::new();

    // paths and operations
    for (_path, item) in &api.paths {
        match item {
            ReferenceOr::Reference { reference } => {
                list.push(RefString::PathItem(reference.to_owned()))
            }
            ReferenceOr::Item(item) => {
                for op in pathitem_operations(&item) {
                    // parameters
                    for prm in &op.parameters {
                        match prm {
                            ReferenceOr::Reference { reference } => {
                                list.push(RefString::Parameter(reference.to_owned()))
                            }
                            ReferenceOr::Item(parameter) => match &parameter.schema {
                                Some(ReferenceOr::Reference { reference }) => {
                                    list.push(RefString::Schema(reference.to_owned()))
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
                            Some(ReferenceOr::Reference { reference }) => {
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
                            ReferenceOr::Reference { reference } => {
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
            ReferenceOr::Reference { reference } => {
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
        let rf = Reference::parse(s)?;
        match rf.file {
            Some(file) => {
                set.insert(file);
                ()
            }
            None => {}
        }
    }

    Ok(set)
}
