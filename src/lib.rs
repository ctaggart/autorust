mod reference;

use autorust_openapi::{OpenAPI, Operation, PathItem, ReferenceOr};
use indexmap::IndexSet;
use reference::Reference;
use std::{fs::File, io::prelude::*};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn read_api_file(path: &str) -> Result<OpenAPI> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    let deserializer = &mut serde_json::Deserializer::from_slice(&bytes);
    let mut ignored: Vec<String> = Vec::new();
    let api: OpenAPI = serde_ignored::deserialize(deserializer, |path| {
        ignored.push(path.to_string());
    })?;
    if ignored.len() > 0 {
        Err(format!("api file {} has some ignored {:?}", path, ignored))?
    }
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

/// returns a set of rererenced files
pub fn get_ref_files(api: &OpenAPI) -> Result<IndexSet<String>> {
    let mut set = IndexSet::new();

    let mut insert = |reference: &str| -> Result<()> {
        let rf = Reference::parse(reference)?;
        match rf.file {
            Some(file) => {
                set.insert(file);
                ()
            }
            None => {}
        }
        Ok(())
    };

    // paths and operations
    for (_path, item) in &api.paths {
        match item {
            ReferenceOr::Reference { reference } => insert(&reference)?,
            ReferenceOr::Item(item) => {
                for op in pathitem_operations(&item) {
                    for prm in &op.parameters {
                        match prm {
                            ReferenceOr::Reference { reference } => insert(&reference)?,
                            _ => {}
                        }
                    }

                    // responses
                    for (_code, rsp) in &op.responses {
                        match &rsp.schema {
                            Some(ReferenceOr::Reference { reference }) => insert(&reference)?,
                            Some(ReferenceOr::Item(_schema)) => {
                                // TODO properties
                                // TODO additionalProperties
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(set)
}
