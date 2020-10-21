use crate::{
    codegen::{create_generated_by_header, ident},
    write_file,
};
use proc_macro2::TokenStream;
use quote::quote;
use std::path::Path;

use crate::Result;

pub fn create(feature_mod_names: &Vec<(String, String)>, path: &Path) -> Result<()> {
    write_file(path, &create_body(feature_mod_names))?;
    Ok(())
}

fn create_body(feature_mod_names: &Vec<(String, String)>) -> TokenStream {
    let mut cfgs = TokenStream::new();
    for (feature_name, mod_name) in feature_mod_names {
        let mod_name = ident(mod_name);
        cfgs.extend(quote! {
            #[cfg(feature = #feature_name)]
            mod #mod_name;
            #[cfg(feature = #feature_name)]
            pub use #mod_name::{models, operations, API_VERSION};
        });
    }
    let generated_by = create_generated_by_header();
    quote! {
        #generated_by
        #cfgs

        pub struct OperationConfig {
            pub api_version: String,
            pub client: reqwest::Client,
            pub base_path: String,
            pub bearer_access_token: Option<String>,
        }

        impl OperationConfig {
            pub fn new(bearer_access_token: &str) -> Self {
                Self {
                    bearer_access_token: Some(bearer_access_token.to_owned()),
                    ..Default::default()
                }
            }
        }

        impl Default for OperationConfig {
            fn default() -> Self {
                Self {
                    api_version: API_VERSION.to_owned(),
                    client: reqwest::Client::new(),
                    base_path: "https://management.azure.com".to_owned(),
                    bearer_access_token: None,
                }
            }
        }

    }
}
