use crate::Result;
use std::{
    fs::File,
    io::{prelude::*, LineWriter},
    path::Path,
};

pub fn create(crate_name: &str, feature_mod_names: &Vec<(String, String)>, path: &Path) -> Result<()> {
    let file = File::create(path)?;
    let mut file = LineWriter::new(file);
    file.write_all(
        format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2018"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
reqwest = {{ version = "0.10", features = ["json"] }}

[dev-dependencies]
tokio = {{ version = "0.2", features = ["macros"] }}

[features]
"#,
            crate_name
        )
        .as_bytes(),
    )?;

    for (i, (feature_name, _mod_name)) in feature_mod_names.iter().enumerate() {
        if i == 0 {
            file.write_all(format!("default = [\"{}\"]\n", feature_name).as_bytes())?;
        }
        file.write_all(format!("{} = []\n", feature_name).as_bytes())?;
    }

    Ok(())
}

// [features]
// default = ["2020-06-01"]
// 2020-06-01 = []
