#![allow(unused_variables, dead_code)]
use crate::{spec, Config, OperationVerb, Reference, ResolvedSchema, Result, Spec};
use autorust_openapi::{DataType, Operation, Parameter, PathItem, ReferenceOr, Schema};
use heck::{CamelCase, SnakeCase};
use indexmap::IndexMap;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use regex::Regex;
use serde_json::Value;
use spec::{get_api_schema_refs, get_schema_schema_refs, RefKey};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

/// code generation context
pub struct CodeGen {
    config: Config,
    pub spec: Spec,
}

impl CodeGen {
    pub fn new(config: Config) -> Result<Self> {
        let spec = Spec::read_files(&config.input_files)?;
        Ok(Self { config, spec })
    }

    pub fn input_files(&self) -> &[PathBuf] {
        &self.config.input_files
    }

    pub fn output_folder(&self) -> &Path {
        &self.config.output_folder
    }

    pub fn api_version(&self) -> Option<&str> {
        self.config.api_version.as_deref()
    }

    // For create_models. Recursively adds schema refs.
    fn add_schema_refs(&self, schemas: &mut IndexMap<RefKey, ResolvedSchema>, doc_file: &Path, schema_ref: &str) -> Result<()> {
        let schema = self.spec.resolve_schema_ref(doc_file, schema_ref)?;
        if let Some(ref_key) = schema.ref_key.clone() {
            if !schemas.contains_key(&ref_key) {
                if !self.spec.is_input_file(&ref_key.file) {
                    let refs = get_schema_schema_refs(&schema.schema);
                    schemas.insert(ref_key.clone(), schema);
                    for rf in refs {
                        self.add_schema_refs(schemas, &ref_key.file, &rf)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn create_models(&self) -> Result<TokenStream> {
        let mut file = TokenStream::new();
        file.extend(create_generated_by_header());
        file.extend(quote! {
            #![allow(non_camel_case_types)]
            #![allow(unused_imports)]
            use crate::*;
            use serde::{Deserialize, Serialize};
        });
        let mut all_schemas: IndexMap<RefKey, ResolvedSchema> = IndexMap::new();

        // all definitions from input_files
        for (doc_file, doc) in &self.spec.docs {
            if self.spec.is_input_file(doc_file) {
                let schemas = self.spec.resolve_schema_map(doc_file, &doc.definitions)?;
                for (name, schema) in schemas {
                    all_schemas.insert(
                        RefKey {
                            file: doc_file.to_owned(),
                            name,
                        },
                        schema,
                    );
                }
            }
        }

        // any referenced schemas from other files
        for (doc_file, doc) in &self.spec.docs {
            if self.spec.is_input_file(doc_file) {
                for rf in get_api_schema_refs(doc) {
                    self.add_schema_refs(&mut all_schemas, doc_file, &rf)?;
                }
            }
        }

        let mut schema_names = IndexMap::new();
        for (ref_key, schema) in &all_schemas {
            let doc_file = &ref_key.file;
            let schema_name = &ref_key.name;
            if let Some(first_doc_file) = schema_names.insert(schema_name, doc_file) {
                eprintln!(
                    "WARN schema {} already created from {:?}, duplicate from {:?}",
                    schema_name, first_doc_file, doc_file
                );
            } else {
                if is_schema_an_array(schema) {
                    file.extend(self.create_vec_alias(doc_file, schema_name, schema)?);
                } else if is_local_enum(schema) {
                    let no_namespace = TokenStream::new();
                    let (_tp_name, tp) = create_enum(&no_namespace, schema_name, schema);
                    file.extend(tp);
                } else {
                    for stream in self.create_struct(doc_file, schema_name, schema)? {
                        file.extend(stream);
                    }
                }
            }
        }
        Ok(file)
    }

    pub fn create_client(&self) -> Result<TokenStream> {
        let mut file = TokenStream::new();
        file.extend(create_generated_by_header());
        file.extend(quote! {
            #![allow(unused_mut)]
            #![allow(unused_variables)]
            use crate::*;
            use anyhow::{Error, Result};
        });
        let param_re = Regex::new(r"\{(\w+)\}").unwrap();
        for (doc_file, doc) in &self.spec.docs {
            let paths = self.spec.resolve_path_map(doc_file, &doc.paths)?;
            for (path, item) in &paths {
                // println!("{}", path);
                for op in spec::pathitem_operations(item) {
                    // println!("{:?}", op.operation_id);
                    file.extend(create_function(self, doc_file, path, item, &op, &param_re))
                }
            }
        }
        Ok(file)
    }

    fn create_vec_alias(&self, doc_file: &Path, alias_name: &str, schema: &ResolvedSchema) -> Result<TokenStream> {
        let items = get_schema_array_items(&schema.schema)?;
        let typ = ident(&alias_name.to_camel_case());
        let items_typ = get_type_name_for_schema_ref(&items)?;
        Ok(quote! { pub type #typ = Vec<#items_typ>; })
    }

    fn create_struct(&self, doc_file: &Path, struct_name: &str, schema: &ResolvedSchema) -> Result<Vec<TokenStream>> {
        // println!("create_struct {} {}", doc_file.to_str().unwrap(), struct_name);
        let mut streams = Vec::new();
        let mut local_types = Vec::new();
        let mut props = TokenStream::new();
        let ns = ident(&struct_name.to_snake_case());
        let nm = ident(&struct_name.to_camel_case());
        let required: HashSet<&str> = schema.schema.required.iter().map(String::as_str).collect();

        for schema in &schema.schema.all_of {
            let type_name = get_type_name_for_schema_ref(schema)?;
            let field_name = ident(&type_name.to_string().to_snake_case());
            props.extend(quote! {
                #[serde(flatten)]
                pub #field_name: #type_name,
            });
        }

        let properties = self.spec.resolve_schema_map(doc_file, &schema.schema.properties)?;
        for (property_name, property) in &properties {
            let nm = ident(&property_name.to_snake_case());
            let (field_tp_name, field_tp) = self.create_struct_field_type(doc_file, &ns, property_name, property)?;
            let is_required = required.contains(property_name.as_str());
            let field_tp_name = require(is_required, field_tp_name);

            if let Some(field_tp) = field_tp {
                local_types.push(field_tp);
            }
            let skip_serialization_if = if is_required {
                quote! {}
            } else {
                quote! {skip_serializing_if = "Option::is_none"}
            };
            let rename = if &nm.to_string() == property_name {
                if is_required {
                    quote! {}
                } else {
                    quote! {#[serde(#skip_serialization_if)]}
                }
            } else {
                if is_required {
                    quote! {#[serde(rename = #property_name)]}
                } else {
                    quote! {#[serde(rename = #property_name, #skip_serialization_if)]}
                }
            };
            props.extend(quote! {
                #rename
                pub #nm: #field_tp_name,
            });
        }

        let st = quote! {
            #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
            pub struct #nm {
                #props
            }
        };
        streams.push(TokenStream::from(st));

        if local_types.len() > 0 {
            let mut types = TokenStream::new();
            local_types.into_iter().for_each(|tp| types.extend(tp));
            streams.push(quote! {
                mod #ns {
                    use super::*;
                    #types
                }
            });
        }

        Ok(streams)
    }

    /// Creates the type reference for a struct field from a struct property.
    /// Optionally, creates a type for a local schema.
    fn create_struct_field_type(
        &self,
        doc_file: &Path,
        namespace: &TokenStream,
        property_name: &str,
        property: &ResolvedSchema,
    ) -> Result<(TokenStream, Option<TokenStream>)> {
        match &property.ref_key {
            Some(ref_key) => {
                let tp = ident(&ref_key.name.to_camel_case());
                Ok((tp, None))
            }
            None => {
                if is_local_enum(property) {
                    let (tp_name, tp) = create_enum(namespace, property_name, property);
                    Ok((tp_name, Some(tp)))
                } else if is_local_struct(property) {
                    let id = ident(&property_name.to_camel_case());
                    let tp_name = quote! {#namespace::#id};
                    let tps = self.create_struct(doc_file, property_name, property)?;
                    Ok((tp_name, Some(tps[0].clone())))
                } else {
                    Ok((get_type_name_for_schema(&property.schema)?, None))
                }
            }
        }
    }
}

fn is_schema_an_array(schema: &spec::ResolvedSchema) -> bool {
    matches!(&schema.schema.common.type_, Some(DataType::Array))
}

fn get_schema_array_items(schema: &Schema) -> Result<&ReferenceOr<Schema>> {
    Ok(schema
        .common
        .items
        .as_ref()
        .as_ref()
        .ok_or_else(|| format!("array expected to have items"))?)
}

fn create_generated_by_header() -> TokenStream {
    let version = env!("CARGO_PKG_VERSION");
    let comment = format!("generated by AutoRust {}", &version);
    quote! { #![doc = #comment] }
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        // https://doc.rust-lang.org/grammar.html#keywords
        "abstract"
            | "alignof"
            | "as"
            | "become"
            | "box"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "do"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "final"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "macro"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "offsetof"
            | "override"
            | "priv"
            | "proc"
            | "pub"
            | "pure"
            | "ref"
            | "return"
            | "Self"
            | "self"
            | "sizeof"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "typeof"
            | "unsafe"
            | "unsized"
            | "use"
            | "virtual"
            | "where"
            | "while"
            | "yield"
    )
}

fn is_local_enum(property: &ResolvedSchema) -> bool {
    property.schema.common.enum_.len() > 0
}

fn is_local_struct(property: &ResolvedSchema) -> bool {
    property.schema.properties.len() > 0
}

fn create_enum(namespace: &TokenStream, property_name: &str, property: &ResolvedSchema) -> (TokenStream, TokenStream) {
    let schema_type = property.schema.common.type_.as_ref();
    let enum_values = enum_values_as_strings(&property.schema.common.enum_);
    let id = ident(&property_name.to_camel_case());
    let mut values = TokenStream::new();
    enum_values.iter().for_each(|name| {
        let nm = ident(&name.to_camel_case());
        let rename = if &nm.to_string() == name {
            quote! {}
        } else {
            quote! { #[serde(rename = #name)] }
        };
        let value = quote! {
            #rename
            #nm,
        };
        values.extend(value);
    });
    let nm = ident(&property_name.to_camel_case());
    let tp = quote! {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        pub enum #nm {
            #values
        }
    };
    let tp_name = quote! {#namespace::#id};
    (tp_name, tp)
}

/// Wraps a type in an Option if is not required.
fn require(is_required: bool, tp: TokenStream) -> TokenStream {
    if is_required {
        tp
    } else {
        quote! { Option<#tp> }
    }
}

fn ident(text: &str) -> TokenStream {
    let text = text.replace(".", "_");
    // prefix with underscore if starts with invalid character
    let text = match text.chars().next().unwrap() {
        '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '0' => format!("_{}", text),
        _ => text.to_owned(),
    };
    let idt = if is_keyword(&text) {
        format_ident!("{}_", text)
    } else {
        format_ident!("{}", text)
    };
    idt.into_token_stream()
}

fn enum_values_as_strings(values: &Vec<Value>) -> Vec<&str> {
    values
        .iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .collect()
}

/// example: pub type Pets = Vec<Pet>;
fn trim_ref(path: &str) -> String {
    let pos = path.rfind('/').map_or(0, |i| i + 1);
    path[pos..].to_string()
}

// simple types in the url
fn map_type(param_type: &DataType) -> TokenStream {
    match param_type {
        DataType::String => quote! { &str },
        DataType::Integer => quote! { i64 },
        _ => quote! { map_type }, // TODO may be Err instead
    }
}

fn get_param_type(param: &Parameter) -> Result<TokenStream> {
    let is_required = param.required.unwrap_or(false);
    let tp = if let Some(param_type) = &param.common.type_ {
        map_type(param_type)
    } else if let Some(schema) = &param.schema {
        get_type_name_for_schema_ref(schema)?
    } else {
        eprintln!("WARN unkown param type for {}", &param.name);
        quote! { serde_json::Value }
    };
    Ok(require(is_required, tp))
}

fn get_param_name_and_type(param: &Parameter) -> Result<TokenStream> {
    let name = ident(&param.name.to_snake_case());
    let typ = get_param_type(param)?;
    Ok(quote! { #name: #typ })
}

fn parse_params(param_re: &Regex, path: &str) -> Vec<String> {
    // capture 0 is the whole match and 1 is the actual capture like other languages
    // param_re.find_iter(path).into_iter().map(|m| m.as_str().to_string()).collect()
    param_re.captures_iter(path).into_iter().map(|c| c[1].to_string()).collect()
}

fn format_path(param_re: &Regex, path: &str) -> String {
    param_re.replace_all(path, "{}").to_string()
}

fn create_function_params(cg: &CodeGen, doc_file: &Path, op: &Operation) -> Result<TokenStream> {
    let parameters: Vec<Parameter> = cg.spec.resolve_parameters(doc_file, &op.parameters)?;
    let mut params: Vec<TokenStream> = Vec::new();
    let mut skip = HashSet::new();
    if cg.api_version().is_some() {
        skip.insert("api-version");
    }
    for param in &parameters {
        if !skip.contains(param.name.as_str()) {
            params.push(get_param_name_and_type(param)?);
        }
    }
    let slf = quote! { configuration: &Configuration };
    params.insert(0, slf);
    Ok(quote! { #(#params),* })
}

fn get_type_name_for_schema(schema: &Schema) -> Result<TokenStream> {
    if let Some(schema_type) = &schema.common.type_ {
        let format = schema.common.format.as_deref();
        let ts = match schema_type {
            DataType::Array => {
                let items = get_schema_array_items(schema)?;
                let vec_items_typ = get_type_name_for_schema_ref(&items)?;
                quote! {Vec<#vec_items_typ>}
            }
            DataType::Integer => {
                if format == Some("int32") {
                    quote! {i32}
                } else {
                    quote! {i64}
                }
            }
            DataType::Number => {
                if format == Some("float") {
                    quote! {f32}
                } else {
                    quote! {f64}
                }
            }
            DataType::String => quote! {String},
            DataType::Boolean => quote! {bool},
            DataType::Object => quote! {serde_json::Value},
        };
        Ok(ts)
    } else {
        eprintln!(
            "WARN unknown type in get_type_name_for_schema, description {:?}",
            schema.common.description
        );
        Ok(quote! {serde_json::Value})
    }
}

fn get_type_name_for_schema_ref(schema: &ReferenceOr<Schema>) -> Result<TokenStream> {
    match schema {
        ReferenceOr::Reference { reference, .. } => {
            let rf = Reference::parse(&reference)?;
            let idt = ident(&rf.name.ok_or_else(|| format!("no name for ref {}", reference))?.to_camel_case());
            Ok(quote! { #idt })
        }
        ReferenceOr::Item(schema) => get_type_name_for_schema(schema),
    }
}

fn create_function_return(verb: &OperationVerb) -> Result<TokenStream> {
    // TODO error responses
    // TODO union of responses
    for (_http_code, rsp) in verb.operation().responses.iter() {
        // println!("response key {:#?} {:#?}", key, rsp);
        if let Some(schema) = &rsp.schema {
            let tp = get_type_name_for_schema_ref(schema)?;
            return Ok(quote! { Result<#tp> });
        }
    }
    Ok(quote! { Result<()> })
}

/// Creating a function name from the path and verb when an operationId is not specified.
/// All azure-rest-api-specs operations should have an operationId.
fn create_function_name(path: &str, verb_name: &str) -> String {
    let mut path = path.split('/').filter(|&x| !x.is_empty()).collect::<Vec<_>>();
    path.push(verb_name);
    path.join("_")
}

fn create_function(
    cg: &CodeGen,
    doc_file: &Path,
    path: &str,
    item: &PathItem,
    operation_verb: &OperationVerb,
    param_re: &Regex,
) -> Result<TokenStream> {
    let fname = ident(
        operation_verb
            .operation()
            .operation_id
            .as_ref()
            .unwrap_or(&create_function_name(path, operation_verb.verb_name()))
            .to_snake_case()
            .as_ref(),
    );

    let params = parse_params(param_re, path);
    // println!("path params {:#?}", params);
    let params: Vec<_> = params.iter().map(|s| ident(&s.to_snake_case())).collect();
    let uri_str_args = quote! { #(#params),* };

    let fpath = format!("{{}}{}", &format_path(param_re, path));
    let fparams = create_function_params(cg, doc_file, operation_verb.operation())?;

    // see if there is a body parameter
    let fresponse = create_function_return(operation_verb)?;

    let client_verb = match operation_verb {
        OperationVerb::Get(_) => quote! { client.get(uri_str) },
        OperationVerb::Post(_) => quote! { client.post(uri_str) },
        OperationVerb::Put(_) => quote! { client.put(uri_str) },
        OperationVerb::Patch(_) => quote! { client.patch(uri_str) },
        OperationVerb::Delete(_) => quote! { client.delete(uri_str) },
        OperationVerb::Options(_) => quote! { client.options(uri_str) },
        OperationVerb::Head(_) => quote! { client.head(uri_str) },
    };

    let mut ts_request_builder = TokenStream::new();
    if let Some(api_version) = cg.api_version() {
        ts_request_builder.extend(quote! {
            if let Some(token) = &configuration.bearer_access_token {
                req_builder = req_builder.query(&[("api-version", &configuration.api_version)]);
            }
        });
    }

    // TODO #17 decode the different errors depending on http status
    // TODO #18 other callbacks like auth
    let func = quote! {
        pub async fn #fname(#fparams) -> #fresponse {
            let client = &configuration.client;
            let uri_str = &format!(#fpath, &configuration.base_path, #uri_str_args);
            let mut req_builder = #client_verb;
            #ts_request_builder
            if let Some(token) = &configuration.bearer_access_token {
                req_builder = req_builder.bearer_auth(token);
            }
            let req = req_builder.build()?;
            let res = client.execute(req).await?;
            match res.error_for_status_ref() {
                Ok(_) => Ok(res.json().await?),
                Err(err) => {
                    let e = Error::new(err);
                    let e = e.context(res.text().await?);
                    Err(e)
                },
            }
        }
    };
    Ok(TokenStream::from(func))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ident_odata_next_link() {
        let idt = "odata.nextLink".to_snake_case();
        assert_eq!(idt, "odata.next_link");
        let idt = ident(&idt);
        assert_eq!(idt.to_string(), "odata_next_link");
    }

    #[test]
    fn test_ident_three_dot_two() {
        let idt = ident("3.2");
        assert_eq!(idt.to_string(), "_3_2");
    }

    #[test]
    fn test_create_function_name() {
        assert_eq!(create_function_name("/pets", "get"), "pets_get");
    }
}
