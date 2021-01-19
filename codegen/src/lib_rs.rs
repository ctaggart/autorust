use crate::{codegen::create_generated_by_header, identifier::ident, write_file};
use proc_macro2::TokenStream;
use quote::quote;
use snafu::{ResultExt, Snafu};
use std::path::Path;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    IdentModNameError {
        source: crate::identifier::Error,
        feature_name: String,
        mod_name: String,
    },
    WriteFileError {
        source: crate::Error,
    },
}

pub fn create(feature_mod_names: &Vec<(String, String)>, path: &Path) -> Result<()> {
    write_file(path, &create_body(feature_mod_names)?).context(WriteFileError)?;
    Ok(())
}

fn create_body(feature_mod_names: &Vec<(String, String)>) -> Result<TokenStream> {
    let mut cfgs = TokenStream::new();
    for (feature_name, mod_name) in feature_mod_names {
        let mod_name = ident(mod_name).context(IdentModNameError {
            feature_name: feature_name.to_owned(),
            mod_name: mod_name.to_owned(),
        })?;
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
            api_version: String,
            http_client: std::sync::Arc<std::boxed::Box<dyn azure_core::HttpClient>>,
            base_path: String,
            token_credential: Option<Box<dyn azure_core::TokenCredential>>,
            token_credential_resource: String,
        }

        impl OperationConfig {
            pub fn new(http_client: std::sync::Arc<std::boxed::Box<dyn azure_core::HttpClient>>, token_credential: Box<dyn azure_core::TokenCredential>) -> Self {
                Self {
                    http_client,
                    api_version: API_VERSION.to_owned(),
                    base_path: "https://management.azure.com".to_owned(),
                    token_credential: Some(token_credential),
                    token_credential_resource: "https://management.azure.com/".to_owned(),
                }
            }

            pub fn api_version(&self) -> &str {
                self.api_version.as_str()
            }

            pub fn http_client(&self) -> &dyn azure_core::HttpClient {
                self.http_client.as_ref().as_ref()
            }

            pub fn base_path(&self) -> &str {
                self.base_path.as_str()
            }

            pub fn token_credential(&self) -> Option<&dyn azure_core::TokenCredential> {
                self.token_credential.as_deref()
            }

            pub fn token_credential_resource(&self) -> &str {
                self.token_credential_resource.as_str()
            }
        }
    })
}
