use snafu::{ResultExt, Snafu};
use std::{
    fs::File,
    io::{prelude::*, LineWriter},
    path::Path,
};

pub type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug, Snafu)]
pub enum Error {
    IoError { source: std::io::Error },
}

pub fn create(crate_name: &str, feature_mod_names: &Vec<(String, String)>, path: &Path) -> Result<()> {
    let file = File::create(path).context(IoError)?;
    let mut file = LineWriter::new(file);
    let version = &env!("CARGO_PKG_VERSION");
    file.write_all(
        format!(
            r#"# generated by AutoRust {}
[package]
name = "{}"
version = "0.1.0"
edition = "2018"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
reqwest = {{ version = "0.10", features = ["json"] }}
bytes = "0.5"
snafu = "0.6"

[dev-dependencies]
tokio = {{ version = "0.2", features = ["macros"] }}

[features]
"#,
            version, crate_name
        )
        .as_bytes(),
    )
    .context(IoError)?;

    let dft = get_default_feature(feature_mod_names);
    file.write_all(format!("default = [\"{}\"]\n", dft).as_bytes()).context(IoError)?;

    for (feature_name, _mod_name) in feature_mod_names {
        file.write_all(format!("\"{}\" = []\n", feature_name).as_bytes()).context(IoError)?;
    }
    Ok(())
}

fn get_default_feature(feature_mod_names: &Vec<(String, String)>) -> String {
    let dft = feature_mod_names
        .iter()
        .map(|(feature, _)| feature)
        .find(|feature| !feature.contains("preview"));
    match dft {
        Some(dft) => dft.clone(),
        None => feature_mod_names[0].0.clone(),
    }
}
