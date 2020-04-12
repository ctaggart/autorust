use heck::{CamelCase, SnakeCase};
use openapi::v2::{Operation, Parameter, ParameterOrRef, PathItem, Schema};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use regex::Regex;
use rustfmt_nightly::{Config, Edition, EmitMode, Input, Session, Verbosity};
use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::prelude::*,
};

fn pathitem_operations(item: &PathItem) -> impl Iterator<Item = &Operation> {
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

fn create_client(spec: &openapi::v2::schema::Spec) -> TokenStream {
    let mut tokens = TokenStream::new();
    if let Some(definitions) = &spec.definitions {
        definitions.iter().for_each(|(name, definition)| {
            for stream in create_struct(name, definition) {
                tokens.extend(stream);
            }
        });
    };
    tokens
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
    ref_path: Option<&str>,
    schema_type: Option<&str>,
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
    let tp = if let Some(ref_path) = ref_path {
        if let Some(i) = ref_path.rfind('/') {
            let id = ident(&ref_path[i + 1..].to_camel_case());
            TokenStream::from(quote! {#id})
        } else {
            let id = ident(&ref_path.to_camel_case());
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
                "array" => {
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

fn create_struct(struct_name: &str, definition: &openapi::v2::schema::Schema) -> Vec<TokenStream> {
    let mut streams = vec![];
    let mut props = TokenStream::new();
    let nm = ident(&struct_name.to_camel_case());
    // println!("definition {:#?}", definition);
    let required: HashSet<String> = if let Some(required) = definition.required.clone() {
        required.into_iter().collect()
    } else {
        HashSet::new()
    };

    if let Some(properties) = &definition.properties {
        properties.iter().for_each(|(name, property)| {
            let nm = ident(&name.to_snake_case());
            // println!("property {:#?}", property);
            // println!("enum_values {:#?}", property.enum_values);
            let is_required = required.contains(name);

            let items: Option<&Schema> = property.items.as_ref().map(Box::as_ref);

            let (tp, inner_tp) = to_rust_type(
                property.ref_path.as_ref().map(String::as_ref),
                property.schema_type.as_ref().map(String::as_ref),
                items,
                property.format.as_ref().map(String::as_ref),
                is_required,
                property
                    .enum_values
                    .as_ref()
                    .map(|v| v.iter().map(String::as_ref).collect()),
                name,
                struct_name,
            );
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
        });
    }
    let st = quote! {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        pub struct #nm {
            #props
        }
    };
    streams.push(TokenStream::from(st));
    streams
}

fn write_file(tokens: &TokenStream, path: &str) {
    // from RLS format.fs
    // https://github.com/rust-lang/rls/blob/master/rls/src/actions/format.rs
    let mut config = Config::default();
    config.set().edition(Edition::Edition2018);
    config.set().emit_mode(EmitMode::Stdout);
    config.set().skip_children(true);
    config.set().verbose(Verbosity::Quiet);
    let mut buf = Vec::<u8>::new();
    {
        let mut session = Session::new(config, Some(&mut buf));
        session.format(Input::Text(tokens.to_string())).unwrap();
    }
    let mut buffer = File::create(path).unwrap();
    buffer.write_all(&buf).unwrap();
    // println!("{:?}", String::from_utf8(buf).unwrap());
}

#[allow(dead_code)]
fn print_operations(spec: &openapi::v2::schema::Spec) {
    // paths and operation
    for (path, item) in &spec.paths {
        println!("{}", path);
        for op in pathitem_operations(item) {
            println!("{:?}", op.operation_id);
        }
    }
}

#[allow(dead_code)]
fn print_definitions(spec: &openapi::v2::schema::Spec) {
    for (name, definition) in spec.definitions.as_ref().unwrap() {
        println!("{}", name);
        println!("{:#?}", definition);
    }
}

#[allow(dead_code)]
fn print_params(spec: &openapi::v2::schema::Spec) {
    if let Some(ref params) = spec.parameters {
        for key in params.keys() {
            println!("param key {:#?}", key);
        }
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

fn get_param_name<'a>(param: &ParameterOrRef, ref_param: &'a RefParam) -> String {
    match param {
        ParameterOrRef::Parameter { name, .. } => name.to_string(),
        ParameterOrRef::Ref { ref_path } => {
            // println!("get_param_name refpath {}", ref_path);
            if let Some(param) = ref_param.ref_param(ref_path) {
                param.name.to_string()
            } else {
                "ref_param_not_found".to_owned()
            }
        }
    }
}

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

fn get_param_type<'a>(param: &ParameterOrRef, ref_param: &'a RefParam) -> TokenStream {
    match param {
        ParameterOrRef::Parameter {
            //required, // TODO
            param_type,
            schema,
            ..
        } => {
            // let required = required.map_or(false);
            // param_type.as_ref().map_or("NoParamType".to_owned(), String::from)
            if let Some(param_type) = param_type {
                map_type(param_type)
            } else if let Some(schema) = schema {
                let tp = get_type_for_schema(schema);
                quote! { &#tp }
            } else {
                let idt = ident("NoParamType1");
                quote! { #idt }
            }
        }
        ParameterOrRef::Ref { ref_path } => {
            // println!("get_param_type refpath {}", ref_path);
            if let Some(param) = ref_param.ref_param(ref_path) {
                if let Some(param_type) = &param.param_type {
                    map_type(param_type)
                } else {
                    let idt = ident("NoParamTyp2");
                    quote! { #idt }
                }
            } else {
                let idt = ident("RefParamNotFound");
                quote! { #idt }
            }
        }
    }
}

fn get_param_name_and_type<'a>(param: &ParameterOrRef, ref_param: &'a RefParam) -> TokenStream {
    let name = ident(&get_param_name(param, ref_param).to_snake_case());
    let typ = get_param_type(param, ref_param);
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

fn create_function_params<'a>(op: &Operation, ref_param: &'a RefParam) -> TokenStream {
    let mut params: Vec<TokenStream> = if let Some(params) = &op.parameters {
        params
            .iter()
            .map(|p| get_param_name_and_type(p, ref_param))
            .collect()
    } else {
        vec![]
    };
    let slf = quote! { &self };
    params.insert(0, slf);
    quote! { #(#params),* }
}

fn get_type_for_schema(schema: &Schema) -> TokenStream {
    // TODO items, schema.enum_values
    let items: Option<&Schema> = schema.items.as_ref().map(Box::as_ref);
    let (tp, _extra) = to_rust_type(
        schema.ref_path.as_deref(),
        schema.schema_type.as_deref(),
        items,
        schema.format.as_deref(),
        true,
        None,
        "PropName",
        "StructName",
    );
    tp
}

// TODO is _ref_param not needed for a return
fn create_function_return<'a>(op: &Operation, _ref_param: &'a RefParam) -> TokenStream {
    // TODO error responses
    // TODO union of respones
    for (_http_code, rsp) in op.responses.iter() {
        // println!("response key {:#?} {:#?}", key, rsp);
        if let Some(schema) = &rsp.schema {
            let tp = get_type_for_schema(schema);
            return quote! { Result<#tp> };
        }
    }
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
    parameters: &'a Option<BTreeMap<String, Parameter>>,
}

impl<'a> RefParam<'a> {
    fn ref_param(&self, rf: &str) -> Option<&'a Parameter> {
        if let Some(parameters) = &self.parameters {
            parameters.get(&trim_ref(rf))
        } else {
            None
        }
    }
}

fn create_api_client(spec: &openapi::v2::schema::Spec) -> TokenStream {
    let mut file = TokenStream::new();
    let param_re = Regex::new(r"\{(\w+)\}").unwrap();
    let ref_param = RefParam {
        parameters: &spec.parameters,
    };

    for (path, item) in &spec.paths {
        // println!("{}", path);
        for op in pathitem_operations(item) {
            // println!("{:?}", op.operation_id);
            file.extend(create_function(&path, &op, &param_re, &ref_param))
        }
    }
    file
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let file = &args[1];
    let fs = File::open(file).expect(&format!("unable to read openapi file {}", file));
    let spec: openapi::v2::schema::Spec =
        serde_yaml::from_reader(fs).expect(&format!("unable to read as v2 schema, file {}", file));

    // print_params(&spec);
    // print_definitions(&spec);
    // print_operations(&spec);

    // TODO combine into single file

    // create model from definitions
    let model = create_client(&spec);
    write_file(&model, "model.rs");

    // create api client from operations
    let client = create_api_client(&spec);
    write_file(&client, "client.rs");
}
