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

        pub struct OperationConfig<'a> {
            api_version: String,
            client: reqwest::Client,
            base_path: String,
            token_credential: Option<&'a dyn azure_core::TokenCredential>,
            token_credential_resource: String,
        }

        impl<'a> OperationConfig<'a> {
            pub fn new(token_credential: &'a dyn azure_core::TokenCredential) -> Self {
                let mut config = Self::default();
                config.set_token_credential(token_credential);
                config
            }
            pub fn set_api_version(&mut self, api_version: String){
                self.api_version = api_version;
            }
            pub fn api_version(&self) -> &str {
                &self.api_version
            }
            pub fn set_client(&mut self, client: reqwest::Client){
                self.client = client;
            }
            pub fn client(&self) -> &reqwest::Client {
                &self.client
            }
            pub fn set_base_path(&mut self, base_path: String){
                self.base_path = base_path;
            }
            pub fn base_path(&self) -> &str {
                &self.base_path
            }
            pub fn set_token_credential(&mut self, token_credential: &'a dyn azure_core::TokenCredential){
                self.token_credential = Some(token_credential);
            }
            pub fn token_credential(&self) -> Option<&'a dyn azure_core::TokenCredential> {
                self.token_credential
            }
            pub fn set_token_credential_resource(&mut self, token_credential_resource: String){
                self.token_credential_resource = token_credential_resource;
            }
            pub fn token_credential_resource(&self) -> &str {
                &self.token_credential_resource
            }
        }

        impl<'a> Default for OperationConfig<'a> {
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
