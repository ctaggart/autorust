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
    pub fn read_file(path: &str) -> Result<Self> {
        let mut docs = IndexMap::new();
        let root = read_api_file(path)?;
        let files = get_ref_files(&root)?;
        docs.insert(path.to_owned(), root);
        for file in files {
            let doc_path = path_join(path, &file)?;
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
            Some(file) => path_join(doc_file, &file)?,
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
            Some(file) => path_join(doc_file, &file)?,
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

    pub fn resolve_schemas(
        &self,
        path: &str,
        schemas: &IndexMap<String, ReferenceOr<Schema>>,
    ) -> Result<IndexMap<String, Schema>> {
        let mut resolved = IndexMap::new();
        for (name, schema) in schemas {
            // let schema =
            match schema {
                ReferenceOr::Reference { reference } => {
                    // TODO resolve
                    let rf = Reference::parse(reference)?;
                    let file = match rf.file {
                        None => path.to_owned(),
                        Some(file) => path_join(path, &file)?,
                    };
                    // let doc = self.docs.get(&path).ok_or_else(|| format!("doc for {}", &path))?;
                    match rf.name {
                        None => Err(format!("no name in reference {}", &reference)),
                        Some(nm) => {
                            let schema = self
                                .schemas
                                .get(&RefKey {
                                    file,
                                    name: nm.clone(),
                                })
                                .ok_or_else(|| format!("a"))?;
                            resolved.insert(name.clone(), schema.clone());
                            Ok(())
                        }
                    }?;
                }
                ReferenceOr::Item(schema) => {
                    resolved.insert(name.clone(), schema.clone());
                }
            }
            // TODO resolve .properties, .additional_properties, .all_of, .items
        }
        Ok(resolved)
    }
}

pub fn resolved_schema_properties(schema: &Schema) -> IndexMap<String, Schema> {
    let mut resolved = IndexMap::new();
    for (name, schema) in &schema.properties {
        match schema {
            ReferenceOr::Reference { reference: _ } => {}
            ReferenceOr::Item(schema) => {
                resolved.insert(name.clone(), *schema.clone());
            }
        }
    }
    resolved
}

pub fn resolved_schema_additonal_properties(schema: &Schema) -> Option<Schema> {
    match &schema.additional_properties {
        Some(ReferenceOr::Reference { reference: _ }) => None,
        Some(ReferenceOr::Item(schema)) => Some(*schema.clone()),
        None => None,
    }
}

pub fn resolved_schema_items(items: &Option<ReferenceOr<Box<Schema>>>) -> Option<Schema> {
    match items {
        Some(ReferenceOr::Reference { reference: _ }) => None,
        Some(ReferenceOr::Item(schema)) => Some(*schema.clone()),
        None => None,
    }
}

pub fn resolved_schema_all_of(all_of: Vec<ReferenceOr<Box<Schema>>>) -> Vec<Schema> {
    let mut resolved = Vec::new();
    for s in all_of {
        match s {
            ReferenceOr::Reference { reference: _ } => {}
            ReferenceOr::Item(schema) => {
                resolved.push(*schema.clone());
            }
        }
    }
    resolved
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
    match &schema.additional_properties {
        Some(ReferenceOr::Reference { reference }) => {
            list.push(RefString::Schema(reference.to_owned()))
        }
        Some(ReferenceOr::Item(schema)) => add_refs_for_schema(list, schema),
        None => {}
    }
    match &schema.items {
        Some(ReferenceOr::Reference { reference }) => {
            list.push(RefString::Schema(reference.to_owned()))
        }
        Some(ReferenceOr::Item(schema)) => add_refs_for_schema(list, schema),
        None => {}
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
