mod reference;

use autorust_codegen::*;
use autorust_openapi::{DataType, OpenAPI, Operation, Parameter, PathItem, ReferenceOr, Schema};
use heck::{CamelCase, SnakeCase};
use indexmap::{IndexMap, IndexSet};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use reference::Reference;
use regex::Regex;
use std::{fs::File, io::prelude::*, path::Path, process::exit};

fn create_client(reader: &ApiReader) -> TokenStream {
    // let mut tokens = TokenStream::new();
    // let definitions = &spec.definitions;
    // definitions.iter().for_each(|(name, definition)| {
    //     for stream in create_struct(name, definition) {
    //         tokens.extend(stream);
    //     }
    // });
    // tokens
    quote! {} // TODO
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
fn to_rust_type(
    ref_uri: Option<&str>,
    schema_type: Option<&DataType>,
    items: Option<&Schema>,
    _schema_format: Option<&str>, // TODO
    required: bool,
    enum_values: Option<Vec<&str>>,
    property_name: &str,
    struct_name: &str,
) -> (TokenStream, Option<TokenStream>) {
    // , enums: Option<Vec<&str>>
    // println!("to_rust_type {:?} {:?} {:?}", ref_path, schema_type, schema_format)
    let mut enum_ts: Option<TokenStream> = None;
    let tp = if let Some(ref_uri) = ref_uri {
        if let Some(i) = ref_uri.rfind('/') {
            let id = ident(&ref_uri[i + 1..].to_camel_case());
            TokenStream::from(quote! {#id})
        } else {
            let id = ident(&ref_uri.to_camel_case());
            TokenStream::from(quote! {#id})
        }
    } else if let Some(enum_values) = enum_values {
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
                    // println!("array items: {:#?}", items);
                    let items = items.expect("array to have item schema");
                    // let item = items.pop().expect("items to not be 0");
                    let vec_items_typ = get_type_for_schema(&items);
                    quote! {Vec<#vec_items_typ>}
                }
                _ => unknown_type,
            }
        } else {
            unknown_type
        }
    };
    if required {
        (tp, enum_ts)
    } else {
        (quote! {Option<#tp>}, enum_ts)
    }
}

fn ident(word: &str) -> Ident {
    if is_keyword(word) {
        format_ident!("{}_", word)
    } else {
        format_ident!("{}", word)
    }
}

// fn create_struct(struct_name: &str, definition: &Schema) -> Vec<TokenStream> {
//     let mut streams = vec![];
//     let mut props = TokenStream::new();
//     let nm = ident(&struct_name.to_camel_case());
//     // println!("definition {:#?}", definition);
//     let required: HashSet<String> = definition.required.into_iter().collect();

//     definition.properties.iter().for_each(|(name, property)| {
//         let nm = ident(&name.to_snake_case());
//         // println!("property {:#?}", property);
//         // println!("enum_ {:#?}", property.enum_);
//         let is_required = required.contains(name);

//         let items: Option<&Schema> = property.items.as_ref().map(Box::as_ref);

//         let (tp, inner_tp) = to_rust_type(
//             property.ref_.as_ref().map(String::as_ref),
//             property.type_.as_ref(),
//             items,
//             property.format.as_ref().map(String::as_ref),
//             is_required,
//             property
//                 .enum_
//                 .as_ref()
//                 .map(|v| v.iter().map(String::as_ref).collect()),
//             name,
//             struct_name,
//         );
//         if let Some(inner_tp) = inner_tp {
//             streams.push(inner_tp);
//         }
//         let skip_serialization_if = if is_required {
//             quote! {}
//         } else {
//             quote! {skip_serializing_if = "Option::is_none"}
//         };
//         let rename = if &nm.to_string() == name {
//             if is_required {
//                 quote! {}
//             } else {
//                 quote! {#[serde(#skip_serialization_if)]}
//             }
//         } else {
//             if is_required {
//                 quote! {#[serde(rename = #name)]}
//             } else {
//                 quote! {#[serde(rename = #name, #skip_serialization_if)]}
//             }
//         };
//         let prop = quote! {
//             #rename
//             #nm: #tp,
//         };
//         props.extend(prop);
//     });

//     let st = quote! {
//         #[derive(Debug, PartialEq, Serialize, Deserialize)]
//         pub struct #nm {
//             #props
//         }
//     };
//     streams.push(TokenStream::from(st));
//     streams
// }

fn format_code(unformatted: String) -> String {
    let mut config = rustfmt_nightly::Config::default();
    config.set().edition(rustfmt_nightly::Edition::Edition2018);
    config.set().max_width(140);
    let setting = rustfmt_nightly::OperationSetting {
        verbosity: rustfmt_nightly::emitter::Verbosity::Quiet,
        ..rustfmt_nightly::OperationSetting::default()
    };
    match rustfmt_nightly::format(
        rustfmt_nightly::Input::Text(unformatted.clone()),
        &config,
        setting,
    ) {
        Ok(report) => match report.format_result().next() {
            Some((_, format_result)) => {
                let formatted = format_result.formatted_text();
                if formatted.is_empty() {
                    unformatted
                } else {
                    formatted.to_owned()
                }
            }
            _ => unformatted,
        },
        Err(_err) => unformatted,
    }
}

fn write_file(tokens: &TokenStream, path: &str) {
    let code = format_code(tokens.to_string());
    let mut buffer = File::create(path).unwrap();
    buffer.write_all(&code.as_bytes()).unwrap();
}

#[allow(dead_code)]
fn print_operations(resolver: &Resolver, reader: &ApiReader) -> Result<()> {
    // paths and operation
    for (path, item) in &reader.api.paths {
        println!("{}", path);
        match item {
            ReferenceOr::Reference { reference } => {
                let rf = Reference::parse(reference);
                println!("  path $ref {:#?}", rf);
            }
            ReferenceOr::Item(item) => {
                for op in pathitem_operations(item) {
                    println!("  {:?}", op.operation_id);

                    // parameters
                    // for param in &op.parameters {
                    //     match param {
                    //         ReferenceOr::Reference { reference } => {
                    //             let rf = Reference::parse(reference);
                    //             println!("    param $ref {:#?}", rf);
                    //         }
                    //         ReferenceOr::Item(prm) => {
                    //             println!("    {:?}", prm.name);
                    //         }
                    //     }
                    // }

                    let params = resolver.resolve_parameters(reader, &op.parameters)?;

                    // responses
                }
            }
        }
    }
    // TODO
    Ok(())
}

#[allow(dead_code)]
fn print_definitions(spec: &OpenAPI) {
    // for (name, definition) in spec.definitions {
    //     println!("{}", name);
    //     println!("{:#?}", definition);
    // }
    // TODO
}

#[allow(dead_code)]
fn print_params(spec: &OpenAPI) {
    for key in spec.parameters.keys() {
        println!("param key {:#?}", key);
    }
}

#[allow(dead_code)]
fn print_responses(spec: &Operation) {
    for (key, rsp) in spec.responses.iter() {
        println!("response key {:#?} {:#?}", key, rsp);
    }
}

fn trim_ref(path: &str) -> String {
    let pos = path.rfind('/').map_or(0, |i| i + 1);
    path[pos..].to_string()
}

// fn get_param_name<'a>(param: &ParameterOrRef, ref_param: &'a RefParam) -> String {
//     match param {
//         ParameterOrRef::Parameter(Parameter { name, .. }) => name.to_string(),
//         ParameterOrRef::Ref(Reference { ref_ }) => {
//             // println!("get_param_name refpath {}", ref_);
//             if let Some(param) = ref_param.ref_param(ref_) {
//                 param.name.to_string()
//             } else {
//                 "ref_param_not_found".to_owned()
//             }
//         }
//     }
// }

// simple types in the url
fn map_type(param_type: &str) -> TokenStream {
    match param_type {
        "string" => quote! { &str },
        "integer" => quote! { i32 },
        _ => {
            let idt = ident(param_type);
            quote! { #idt }
        }
    }
}

// fn get_param_type<'a>(param: &ParameterOrRef, ref_param: &'a RefParam) -> TokenStream {
//     match param {
//         ParameterOrRef::Parameter(Parameter {
//             //required, // TODO
//             type_,
//             schema,
//             ..
//         }) => {
//             // let required = required.map_or(false);
//             // type_.as_ref().map_or("NoParamType".to_owned(), String::from)
//             if let Some(param_type) = type_ {
//                 map_type(param_type)
//             } else if let Some(schema) = schema {
//                 let tp = get_type_for_schema(schema);
//                 quote! { &#tp }
//             } else {
//                 let idt = ident("NoParamType1");
//                 quote! { #idt }
//             }
//         }
//         ParameterOrRef::Ref(Reference { ref_ }) => {
//             // println!("get_param_type refpath {}", ref_);
//             if let Some(param) = ref_param.ref_param(ref_) {
//                 if let Some(param_type) = &param.type_ {
//                     map_type(param_type)
//                 } else {
//                     let idt = ident("NoParamType2");
//                     quote! { #idt }
//                 }
//             } else {
//                 let idt = ident("RefParamNotFound");
//                 quote! { #idt }
//             }
//         }
//     }
// }

// fn get_param_name_and_type<'a>(param: &ParameterOrRef, ref_param: &'a RefParam) -> TokenStream {
//     let name = ident(&get_param_name(param, ref_param).to_snake_case());
//     let typ = get_param_type(param, ref_param);
//     quote! { #name: #typ }
// }

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

fn create_function_params<'a>(op: &Operation, ref_param: &'a RefParam) -> TokenStream {
    // let mut params: Vec<TokenStream> =
    //     op.parameters
    //         .iter()
    //         .map(|p| get_param_name_and_type(p, ref_param))
    //         .collect();

    // let slf = quote! { &self };
    // params.insert(0, slf);
    // quote! { #(#params),* }
    quote! {} // TODO
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
fn create_function_return<'a>(op: &Operation, _ref_param: &'a RefParam) -> TokenStream {
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

fn create_function<'a>(
    path: &str,
    op: &Operation,
    param_re: &Regex,
    ref_param: &'a RefParam,
) -> TokenStream {
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
    let fparams = create_function_params(op, ref_param);

    // see if there is a body parameter
    // print_responses(&op);
    let fresponse = create_function_return(op, ref_param);

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
    TokenStream::from(func)
}

struct RefParam<'a> {
    parameters: &'a IndexMap<String, Parameter>,
}

impl<'a> RefParam<'a> {
    fn ref_param(&self, rf: &str) -> Option<&'a Parameter> {
        self.parameters.get(&trim_ref(rf))
    }
}

fn create_api_client(reader: &ApiReader) -> TokenStream {
    // let mut file = TokenStream::new();
    // let param_re = Regex::new(r"\{(\w+)\}").unwrap();
    // let ref_param = RefParam {
    //     parameters: &spec.parameters,
    // };

    // for (path, item) in &spec.paths {
    //     // println!("{}", path);
    //     for op in pathitem_operations(item) {
    //         // println!("{:?}", op.operation_id);
    //         file.extend(create_function(&path, &op, &param_re, &ref_param))
    //     }
    // }
    // file
    quote! {} // TODO
}

type ApiCache = IndexMap<String, ApiReader>;
pub struct Resolver(ApiCache);

#[derive(Clone, Debug)]
pub struct ApiReader {
    pub path: String,
    pub api: OpenAPI,
}

impl ApiReader {
    // fn read_file<'a>(resolver: &'a mut Resolver, path: &str) -> Result<&'a ApiReader>{
    fn read_file(resolver: &mut Resolver, path: &str) -> Result<ApiReader> {
        let cache = &mut resolver.0;
        // let reader = cache.get_mut TODO avoid second lookup
        match cache.get_mut(path) {
            Some(_reader) => (),
            None => {
                let mut bytes = Vec::new();
                File::open(path)?.read_to_end(&mut bytes)?;
                let deserializer = &mut serde_json::Deserializer::from_slice(&bytes);
                let api: OpenAPI = serde_ignored::deserialize(deserializer, |path| {
                    // TODO remove side effects
                    println!("ignored {}", path.to_string());
                })?;
                cache.insert(
                    path.to_string(),
                    ApiReader {
                        path: path.to_string(),
                        api,
                    },
                );
            }
        }
        match cache.get(path) {
            Some(reader) => Ok(reader.to_owned()),
            None => Err("not in cache")?,
        }
    }

    fn read_parameter(param: &ReferenceOr<Parameter>) -> Result<Parameter> {
        Err("TODO")?
    }
}

impl Resolver {
    // fn read_file(&mut self, path: &str) -> Result<ApiReader>{
    //     let cache = &mut self.0;
    //     // let reader = cache.get_mut TODO avoid second lookup
    //     match cache.get_mut(path){
    //         Some(_reader) => (),
    //         None => {
    //             let mut bytes = Vec::new();
    //             File::open(path)?.read_to_end(&mut bytes)?;
    //             let deserializer = &mut serde_json::Deserializer::from_slice(&bytes);
    //             let api: OpenAPI = serde_ignored::deserialize(deserializer, |path| {
    //                 // TODO remove side effects
    //                 println!("ignored {}", path.to_string());
    //             })?;
    //             cache.insert(path.to_string(), ApiReader { path: path.to_string(), api});
    //         }
    //     }
    //     match cache.get(path){
    //         Some(reader) => Ok(reader.to_owned()),
    //         None => Err("not in cache")?
    //     }
    // }

    fn resolve_parameters(
        &self,
        reader: &ApiReader,
        parameters: &Vec<ReferenceOr<Parameter>>,
    ) -> Result<Vec<Parameter>> {
        Err("TODO")?
        // let mut resolved = Vec::new();
        // for param in parameters {
        //     match param {
        //         ReferenceOr::Item(param) => resolved.push(param.clone()),
        //         ReferenceOr::Reference { reference } => {
        //             let rf = Reference::parse(reference)?;
        //             println!("    param $ref {:#?}", rf);

        //             let rdr =
        //                 match rf.file {
        //                     Some(file) => {
        //                         // TODO combine path
        //                         let path = Path::new(&reader.path).join(file);
        //                         let path = path.to_str().unwrap();
        //                         // self.read_file(path)?
        //                     },
        //                     None => reader.to_owned(),
        //                 };

        //         }

        //     }
        // }
        // Ok(resolved)
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("path to spec required");
        exit(1);
    }
    let path = &args[1];

    // let mut cache = ApiCache::new();
    let mut resolver = &mut Resolver(ApiCache::new());
    let reader = &ApiReader::read_file(&mut resolver, &path)?;

    // let ref_files = reader.ref_files()?;
    // println!("ref_files {:?}", ref_files);

    // let reader = &resolver.read_file(&path)?;
    // let api = &reader.api;

    // print_params(&spec);
    // print_definitions(&spec);
    print_operations(resolver, reader)?;

    // TODO combine into single file

    // create model from definitions
    let model = create_client(reader);
    write_file(&model, "model.rs");

    // create api client from operations
    let client = create_api_client(reader);
    write_file(&client, "client.rs");

    Ok(())
}
