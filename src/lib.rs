mod path;
mod reference;

use autorust_openapi::{OpenAPI, Operation, PathItem, ReferenceOr, Schema};
use indexmap::IndexSet;
use reference::Reference;
use std::{fs::File, io::prelude::*};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

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
