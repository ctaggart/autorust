#![allow(unused_variables, dead_code)]
use crate::{format_code, pathitem_operations, Reference, Result, Spec};
use autorust_openapi::{DataType, Operation, Parameter, ReferenceOr, Schema};
use heck::{CamelCase, SnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use regex::Regex;
use std::{collections::HashSet, fs::File, io::prelude::*};

/// code generation context
pub struct CodeGen {
    pub spec: Spec,
}

pub fn create_models(cg: &CodeGen) -> Result<TokenStream> {
    let mut tokens = TokenStream::new();
    let (root_path, root_doc) = cg.spec.docs.get_index(0).unwrap();
    let schemas = &cg
        .spec
        .resolve_schema_map(root_path, &root_doc.definitions)?;
    for (name, schema) in schemas {
        for stream in create_struct(cg, root_path, name, schema)? {
            tokens.extend(stream);
        }
    }
    Ok(tokens)
}

fn is_keyword(word: &str) -> bool {
    match word {
        // https://doc.rust-lang.org/grammar.html#keywords
        "abstract" | "alignof" | "as" | "become" | "box" | "break" | "const" | "continue"
        | "crate" | "do" | "else" | "enum" | "extern" | "false" | "final" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "macro" | "match" | "mod" | "move" | "mut"
        | "offsetof" | "override" | "priv" | "proc" | "pub" | "pure" | "ref" | "return"
        | "Self" | "self" | "sizeof" | "static" | "struct" | "super" | "trait" | "true"
        | "type" | "typeof" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while"
        | "yield" => true,
        _ => false,
    }
}

fn create_enum(struct_name: &str, property_name: &str, enum_values: Vec<&str>) -> TokenStream {
    let mut values = TokenStream::new();

    enum_values.iter().for_each(|name| {
        let nm = ident(&name.to_camel_case());
        let rename = if &nm.to_string() == name {
            quote! {}
        } else {
            quote! {#[serde(rename = #name)]}
        };
        let value = quote! {
            #rename
            #nm,
        };
        values.extend(value);
    });

    let ns = ident(&struct_name.to_snake_case());
    let nm = ident(&property_name.to_camel_case());

    let enm = quote! {
        mod #ns {
            #[derive(Debug, PartialEq, Serialize, Deserialize)]
            pub enum #nm {
                #values
            }
        }
    };

    TokenStream::from(enm)
}

// type: "string", "array", "integer"
// format: "uuid", "date-time", "i32"
fn create_struct_field_type(
    schema_type: Option<&DataType>,
    items: &Option<ReferenceOr<Schema>>,
    _schema_format: Option<&str>, // TODO
    required: bool,
    enum_values: Vec<&str>,
    property_name: &str,
    struct_name: &str,
) -> Result<(TokenStream, Option<TokenStream>)> {
    let mut enum_ts: Option<TokenStream> = None;
    let tp = if enum_values.len() > 0 {
        enum_ts = Some(create_enum(struct_name, property_name, enum_values));
        let ns = ident(&struct_name.to_snake_case());
        let id = ident(&property_name.to_camel_case());
        TokenStream::from(quote! {#ns::#id})
    } else {
        let unknown_type = quote!(UnknownType);
        if let Some(schema_type) = schema_type {
            match schema_type {
                DataType::Array => {
                    let items = items.as_ref().ok_or_else(|| {
                        format!(
                            "array expected to have items, struct {}, property {}",
                            struct_name, property_name
                        )
                    })?;
                    let vec_items_typ = get_type_for_schema(&items)?;
                    quote! {Vec<#vec_items_typ>}
                }
                DataType::Integer => quote! {i32},
                DataType::Number => quote! {f64},
                DataType::String => quote! {String},
                DataType::Boolean => quote! {bool},
                _ => unknown_type,
            }
        } else {
            unknown_type
        }
    };
    if required {
        Ok((tp, enum_ts))
    } else {
        Ok((quote! {Option<#tp>}, enum_ts))
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

fn create_struct(
    cg: &CodeGen,
    doc_file: &str,
    struct_name: &str,
    schema: &Schema,
) -> Result<Vec<TokenStream>> {
    let mut streams = vec![];
    let mut props = TokenStream::new();
    let nm = ident(&struct_name.to_camel_case());
    let required: HashSet<&str> = schema.required.iter().map(String::as_str).collect();

    let properties = cg.spec.resolve_schema_map(doc_file, &schema.properties)?;
    for (name, property) in &properties {
        let nm = ident(&name.to_snake_case());
        let is_required = required.contains(name.as_str());
        let enum_values: Vec<&str> = property.enum_.iter().map(String::as_str).collect();

        let (tp, inner_tp) = create_struct_field_type(
            property.type_.as_ref(),
            &property.items,
            property.format.as_ref().map(String::as_ref),
            is_required,
            enum_values,
            name,
            struct_name,
        )?;
        if let Some(inner_tp) = inner_tp {
            streams.push(inner_tp);
        }
        let skip_serialization_if = if is_required {
            quote! {}
        } else {
            quote! {skip_serializing_if = "Option::is_none"}
        };
        let rename = if &nm.to_string() == name {
            if is_required {
                quote! {}
            } else {
                quote! {#[serde(#skip_serialization_if)]}
            }
        } else {
            if is_required {
                quote! {#[serde(rename = #name)]}
            } else {
                quote! {#[serde(rename = #name, #skip_serialization_if)]}
            }
        };
        let prop = quote! {
            #rename
            #nm: #tp,
        };
        props.extend(prop);
    }

    let st = quote! {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        pub struct #nm {
            #props
        }
    };
    streams.push(TokenStream::from(st));
    Ok(streams)
}

pub fn write_file(tokens: &TokenStream, path: &str) {
    let code = format_code(tokens.to_string());
    let mut buffer = File::create(path).unwrap();
    buffer.write_all(&code.as_bytes()).unwrap();
}

fn trim_ref(path: &str) -> String {
    let pos = path.rfind('/').map_or(0, |i| i + 1);
    path[pos..].to_string()
}

// simple types in the url
fn map_type(param_type: &DataType) -> TokenStream {
    match param_type {
        DataType::String => quote! { &str },
        DataType::Integer => quote! { i32 },
        _ => {
            quote! {map_type} // TODO may be Err instead
        }
    }
}

fn get_param_type(param: &Parameter) -> Result<TokenStream> {
    // let required = required.map_or(false); // TODO
    if let Some(param_type) = &param.type_ {
        Ok(map_type(param_type))
    } else if let Some(schema) = &param.schema {
        Ok(get_type_for_schema(schema)?)
    } else {
        let idt = ident("NoParamType1");
        Ok(quote! { #idt }) // TOOD may be Err instead
    }
}

fn get_param_name_and_type(param: &Parameter) -> Result<TokenStream> {
    let name = ident(&param.name.to_snake_case());
    let typ = get_param_type(param)?;
    Ok(quote! { #name: #typ })
}

fn parse_params(param_re: &Regex, path: &str) -> Vec<String> {
    // capture 0 is the whole match and 1 is the actual capture like other languages
    // param_re.find_iter(path).into_iter().map(|m| m.as_str().to_string()).collect()
    param_re
        .captures_iter(path)
        .into_iter()
        .map(|c| c[1].to_string())
        .collect()
}

fn format_path(param_re: &Regex, path: &str) -> String {
    param_re.replace_all(path, "{}").to_string()
}

fn create_function_params(cg: &CodeGen, op: &Operation) -> Result<TokenStream> {
    let doc_file = cg.spec.root_file(); // TODO pass in
    let parameters: Vec<Parameter> = cg.spec.resolve_parameters(doc_file, &op.parameters)?;
    let mut params: Vec<TokenStream> = Vec::new();
    for param in &parameters {
        params.push(get_param_name_and_type(param)?);
    }
    let slf = quote! { &self };
    params.insert(0, slf);
    Ok(quote! { #(#params),* })
}

fn get_type_for_schema(schema: &ReferenceOr<Schema>) -> Result<TokenStream> {
    match schema {
        ReferenceOr::Reference { reference, .. } => {
            let rf = Reference::parse(&reference)?;
            let idt = ident(
                &rf.name
                    .ok_or_else(|| format!("no name for ref {}", reference))?,
            );
            Ok(quote! { #idt })
        }
        ReferenceOr::Item(_) => {
            // TODO probably need to create a struct
            // and have a way to name it
            let idt = ident("NoParamType2");
            Ok(quote! { #idt })
        }
    }
}

// TODO is _ref_param not needed for a return
fn create_function_return(op: &Operation) -> Result<TokenStream> {
    // TODO error responses
    // TODO union of responses
    for (_http_code, rsp) in op.responses.iter() {
        // println!("response key {:#?} {:#?}", key, rsp);
        if let Some(schema) = &rsp.schema {
            let tp = get_type_for_schema(schema)?;
            return Ok(quote! { Result<#tp> });
        }
    }
    Ok(quote! { Result<()> })
}

fn create_function(
    cg: &CodeGen,
    path: &str,
    op: &Operation,
    param_re: &Regex,
) -> Result<TokenStream> {
    let name_default = "operation_id_missing";
    let name = ident(
        op.operation_id
            .as_ref()
            .map(String::as_ref)
            .unwrap_or(name_default),
    );

    let params = parse_params(param_re, path);
    // println!("path params {:#?}", params);
    let params: Vec<_> = params.iter().map(|s| ident(&s.to_snake_case())).collect();
    let uri_str_args = quote! { #(#params),* };

    let fpath = format!("{{}}{}", &format_path(param_re, path));

    // get path parameters
    // Option if not required
    let fparams = create_function_params(cg, op)?;

    // see if there is a body parameter
    let fresponse = create_function_return(op)?;

    let func = quote! {
        pub async fn #name(#fparams) -> #fresponse {
            let configuration = self.configuration;
            let client = &configuration.client;
            let uri_str = format!(#fpath, configuration.base_path, #uri_str_args);
            // TODO client.get, put, post, delete
            let mut req_builder = client.get(uri_str.as_str());
            // TODO other callbacks like auth
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

pub fn create_client(cg: &CodeGen) -> Result<TokenStream> {
    let mut file = TokenStream::new();
    let param_re = Regex::new(r"\{(\w+)\}").unwrap();
    let (doc_file, doc) = cg.spec.root();
    let paths = cg.spec.resolve_path_map(doc_file, &doc.paths)?;
    for (path, item) in &paths {
        // println!("{}", path);
        for op in pathitem_operations(item) {
            // println!("{:?}", op.operation_id);
            file.extend(create_function(cg, &path, &op, &param_re))
        }
    }
    Ok(file)
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
}
