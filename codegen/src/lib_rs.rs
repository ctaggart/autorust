use crate::{
    codegen::{create_generated_by_header, ident},
    write_file,
};
use proc_macro2::TokenStream;
use quote::quote;
use snafu::{ResultExt, Snafu};
use std::path::Path;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    CodeGenError { source: crate::codegen::Error },
    WriteFileError { source: crate::Error },
}

pub fn create(feature_mod_names: &Vec<(String, String)>, path: &Path) -> Result<()> {
    write_file(path, &create_body(feature_mod_names)?).context(WriteFileError)?;
    Ok(())
}

fn create_body(feature_mod_names: &Vec<(String, String)>) -> Result<TokenStream> {
    let mut cfgs = TokenStream::new();
    for (feature_name, mod_name) in feature_mod_names {
        let mod_name = ident(mod_name).context(CodeGenError)?;
        cfgs.extend(quote! {
            #[cfg(feature = #feature_name)]
            mod #mod_name;
            #[cfg(feature = #feature_name)]
            pub use #mod_name::{models, operations, API_VERSION};
        });
    }
    let generated_by = create_generated_by_header();
    Ok(quote! {
        #generated_by
        #cfgs

        pub struct OperationConfig {
            pub api_version: String,
            pub client: reqwest::Client,
            pub base_path: String,
            pub token_credential: Option<Box<dyn azure_core::TokenCredential>>,
            pub token_credential_resource: String,
        }

        impl OperationConfig {
            pub fn new(token_credential: Box<dyn azure_core::TokenCredential>) -> Self {
                Self {
                    token_credential: Some(token_credential),
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
                    token_credential: None,
                    token_credential_resource: "https://management.azure.com/".to_owned(),
                }
            }
        }
    })
}
