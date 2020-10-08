use crate::Result;
use clap::{App, Arg, ArgMatches};
use std::path::{Path, PathBuf};

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const INPUT_FILE: &str = "input-file";
const OUTPUT_FOLDER: &str = "output-folder";
const GENERATED: &str = "generated";

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    input_files: Vec<PathBuf>,
    output_folder: PathBuf,
}

impl Config {
    pub fn try_new() -> Result<Config> {
        let arg_matches = new_app().get_matches();
        Self::try_new_from_matches(&arg_matches)
    }

    fn try_new_from_matches(arg_matches: &ArgMatches) -> Result<Config> {
        let input_files = arg_matches
            .values_of(INPUT_FILE)
            .unwrap()
            .map(|s| s.into())
            .collect::<Vec<_>>();
        let output_folder = arg_matches
            .value_of(OUTPUT_FOLDER)
            .unwrap()
            .to_owned()
            .into();
        Ok(Config {
            input_files,
            output_folder,
        })
    }

    pub fn input_files(&self) -> &[PathBuf] {
        &self.input_files
    }

    pub fn output_folder(&self) -> &Path {
        &self.output_folder
    }
}

fn new_app() -> App<'static> {
    App::new(NAME)
    .version(VERSION)
    .arg(Arg::new(INPUT_FILE)
        .about("OpenAPI file to use as input (use this setting repeatedly to pass multiple files at once)")
        .long(INPUT_FILE)
        .required(true)
        .takes_value(true)
        .multiple(true))
    .arg(Arg::new(OUTPUT_FOLDER)
        .about("target folder for generated artifacts; default: \"<base folder>/generated\"")
        .long(OUTPUT_FOLDER)
        .takes_value(true)
        .default_value(GENERATED))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ErrorKind;

    #[test]
    fn missing_required() {
        let m = new_app().try_get_matches_from(vec![""]);
        assert!(m.is_err(), "{:?}", m);
        assert_eq!(m.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn two_input_files() {
        let m = new_app().try_get_matches_from(vec![
            NAME,
            "--input-file",
            "abc.json",
            "--input-file",
            "def.json",
        ]);
        assert!(m.is_ok(), "{:?}", m);
        let m = m.unwrap();
        assert_eq!(m.occurrences_of(INPUT_FILE), 2);
        assert_eq!(
            m.values_of(INPUT_FILE).unwrap().collect::<Vec<_>>(),
            ["abc.json", "def.json"]
        );
        assert_eq!(m.value_of(OUTPUT_FOLDER).unwrap(), GENERATED);
    }

    #[test]
    fn args_with_equals() {
        let m = new_app().try_get_matches_from(vec![
            NAME,
            "--input-file=abc.json",
            "--input-file=def.json",
            "--output-folder=src",
        ]);
        assert!(m.is_ok(), "{:?}", m);
        let m = m.unwrap();
        assert_eq!(m.occurrences_of(INPUT_FILE), 2);
        assert_eq!(
            m.values_of(INPUT_FILE).unwrap().collect::<Vec<_>>(),
            ["abc.json", "def.json"]
        );
        assert_eq!(m.value_of(OUTPUT_FOLDER).unwrap(), "src");
    }

    #[test]
    fn test_new_config() -> Result<()> {
        let m = new_app().try_get_matches_from(vec![
            NAME,
            "--input-file=abc.json",
            "--input-file=def.json",
            "--output-folder=src",
        ]);
        assert!(m.is_ok(), "{:?}", m);
        let m = m?;
        let c = Config::try_new_from_matches(&m)?;
        let input_files: [PathBuf; 2] = ["abc.json".into(), "def.json".into()];
        assert_eq!(c.input_files, input_files);
        let output_folder: PathBuf = "src".into();
        assert_eq!(c.output_folder, output_folder);
        Ok(())
    }
}
