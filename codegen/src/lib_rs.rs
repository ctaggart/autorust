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

pub fn create(crate_name: &str, feature_mod_names: &Vec<(String, String)>, path: &Path) -> Result<()> {
    write_file(path, &create_body(crate_name, feature_mod_names)?).context(WriteFileError)?;
    Ok(())
}

fn create_body(crate_name: &str, feature_mod_names: &Vec<(String, String)>) -> Result<TokenStream> {
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
    let user_agent_sdk = format!("azsdk-rust-{}/0.1.0", crate_name);
    Ok(quote! {
        #generated_by
        pub const USER_AGENT_SDK: &str = #user_agent_sdk;
        #cfgs

        #[derive(Clone)]
        pub struct OperationConfig {
            http_client: azure_core::HttpClientArc,
            token_credential: azure_core::TokenCredentialArc,
            token_credential_resource: String,
            base_path: String,
            api_version: String,
            user_agent: Option<String>,
        }

        impl OperationConfig {
            pub fn new(http_client: azure_core::HttpClientArc, token_credential: azure_core::TokenCredentialArc) -> Self {
                Self {
                    http_client,
                    token_credential,
                    token_credential_resource: "https://management.azure.com/".to_owned(),
                    base_path: "https://management.azure.com".to_owned(),
                    api_version: API_VERSION.to_owned(),
                    user_agent: Some(format!("{} ({})", USER_AGENT_SDK, std::env::consts::ARCH)),
                }
            }
            pub fn new_all(
                http_client: azure_core::HttpClientArc,
                token_credential: azure_core::TokenCredentialArc,
                token_credential_resource: String,
                base_path: String,
                api_version: String,
                user_agent: Option<String>,
            ) -> Self {
                Self {
                    http_client,
                    token_credential,
                    token_credential_resource,
                    base_path,
                    api_version,
                    user_agent,
                }
            }
            pub fn http_client(&self) -> &reqwest::Client {
                self.http_client.as_ref()
            }
            pub fn token_credential(&self) -> &dyn azure_core::TokenCredential {
                self.token_credential.as_ref().as_ref()
            }
            pub fn set_token_credential_resource(&mut self, token_credential_resource: String){
                self.token_credential_resource = token_credential_resource;
            }
            pub fn token_credential_resource(&self) -> &str {
                &self.token_credential_resource
            }
            pub fn set_base_path(&mut self, base_path: String){
                self.base_path = base_path;
            }
            pub fn base_path(&self) -> &str {
                &self.base_path
            }
            pub fn set_api_version(&mut self, api_version: String){
                self.api_version = api_version;
            }
            pub fn api_version(&self) -> &str {
                &self.api_version
            }
            pub fn user_agent(&self) -> Option<&str> {
                self.user_agent.as_deref()
            }
            pub fn set_user_agent(&mut self, user_agent: Option<String>){
                self.user_agent = user_agent;
            }
        }

    })
}
