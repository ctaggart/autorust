#![allow(unused_variables, dead_code)]
use crate::{format_code, pathitem_operations, Result, Spec};
use autorust_openapi::{DataType, Operation, Parameter, Schema};
use heck::{CamelCase, SnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use regex::Regex;
use std::{collections::HashSet, fs::File, io::prelude::*};

/// code generation context
pub struct CodeGen {
    pub spec: Spec,
}

pub fn create_model(cg: &CodeGen) -> Result<TokenStream> {
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
    items: Option<&Schema>,
    _schema_format: Option<&str>, // TODO
    required: bool,
    enum_values: Vec<&str>,
    property_name: &str,
    struct_name: &str,
) -> Result<(TokenStream, Option<TokenStream>)> {
    // , enums: Option<Vec<&str>>
    // println!("to_rust_type {:?} {:?} {:?}", ref_path, schema_type, schema_format)
    let mut enum_ts: Option<TokenStream> = None;

    let tp = if enum_values.len() > 0 {
        enum_ts = Some(create_enum(struct_name, property_name, enum_values));
        let ns = ident(&struct_name.to_snake_case());
        let id = ident(&property_name.to_camel_case());
        TokenStream::from(quote! {#ns::#id})
    } else {
        // TODO array, integer
        let unknown_type = quote!(UnknownType);

        if let Some(schema_type) = schema_type {
            match schema_type {
                DataType::Array => {
                    // println!("struct_name: {:#?}", struct_name);
                    // println!("array items: {:#?}", items);
                    // let items = items.expect("array to have item schema");
                    let items = items.ok_or_else(|| {
                        format!(
                            "array expected to have items, struct {}, property {}",
                            struct_name, property_name
                        )
                    })?;
                    // let item = items.pop().expect("items to not be 0");
                    let vec_items_typ = get_type_for_schema(&items);
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

fn ident(word: &str) -> Ident {
    if is_keyword(word) {
        format_ident!("{}_", word)
    } else {
        format_ident!("{}", word)
    }
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
    // println!("definition {:#?}", definition);
    let required: HashSet<&str> = schema.required.iter().map(String::as_str).collect();

    let properties = cg
        .spec
        .resolve_box_schema_map(doc_file, &schema.properties)?;
    for (name, property) in &properties {
        let nm = ident(&name.to_snake_case());
        // println!("property {:#?}", property);
        // println!("enum_ {:#?}", property.enum_);
        let is_required = required.contains(name.as_str());

        let items = match &property.items {
            None => None,
            Some(items) => Some(cg.spec.resolve_box_schema(doc_file, items)?),
        };

        let enum_values: Vec<&str> = property.enum_.iter().map(String::as_str).collect();

        let (tp, inner_tp) = create_struct_field_type(
            property.type_.as_ref(),
            items.as_ref(),
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
            // let idt = ident(param_type);
            // quote! { #idt }
            quote! {map_type}
        }
    }
}

fn get_param_type<'a>(param: &Parameter) -> TokenStream {
    // let required = required.map_or(false);
    if let Some(param_type) = &param.type_ {
        map_type(param_type)
    // } else if let Some(schema) = param.schema { // TODO
    //     let tp = get_type_for_schema(schema);
    //     quote! { &#tp }
    } else {
        let idt = ident("NoParamType1");
        quote! { #idt }
    }
}

fn get_param_name_and_type(param: &Parameter) -> TokenStream {
    let name = ident(&param.name.to_snake_case());
    let typ = get_param_type(param);
    quote! { #name: #typ }
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
    let params: Vec<Parameter> = cg.spec.resolve_parameters(doc_file, &op.parameters)?;
    let mut params: Vec<TokenStream> = params.iter().map(|p| get_param_name_and_type(p)).collect();

    let slf = quote! { &self };
    params.insert(0, slf);
    Ok(quote! { #(#params),* })
}

fn get_type_for_schema(schema: &Schema) -> TokenStream {
    // // TODO items, schema.enum_
    // let items: Option<&Schema> = schema.items.as_ref().map(Box::as_ref);
    // let (tp, _extra) = to_rust_type(
    //     schema.ref_.as_deref(),
    //     schema.type_.as_ref(),
    //     items,
    //     schema.format.as_deref(),
    //     true,
    //     None,
    //     "PropName",
    //     "StructName",
    // );
    // tp
    quote! {} // TODO
}

// TODO is _ref_param not needed for a return
fn create_function_return(op: &Operation) -> TokenStream {
    // TODO error responses
    // TODO union of respones
    // for (_http_code, rsp) in op.responses.iter() {
    //     // println!("response key {:#?} {:#?}", key, rsp);
    //     if let Some(schema) = &rsp.schema {
    //         let tp = get_type_for_schema(schema);
    //         return quote! { Result<#tp> };
    //     }
    // }
    quote! { Result<()> }
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

    // println!("path {}", path);
    // println!("params {:#?}", op.parameters);

    // let param_names: Vec<String> = if let Some(params) = &op.parameters {
    //     params
    //         .iter()
    //         .map(|p| get_param_name(p, ref_param))
    //         .collect()
    // } else {
    //     vec![]
    // };
    // println!("param names {:#?}", param_names);

    let params = parse_params(param_re, path);
    // println!("path params {:#?}", params);
    let params: Vec<Ident> = params.iter().map(|s| ident(&s.to_snake_case())).collect();
    let uri_str_args = quote! { #(#params),* };

    let fpath = format!("{{}}{}", &format_path(param_re, path));

    // get path parameters
    // Option if not required
    let fparams = create_function_params(cg, op)?;

    // see if there is a body parameter
    // print_responses(&op);
    let fresponse = create_function_return(op);

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
