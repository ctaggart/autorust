#[cfg(feature = "fmt")]
pub fn format_code(unformatted: String) -> String {
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

#[cfg(not(feature = "fmt"))]
pub fn format_code(unformatted: String) -> String {
    unformatted
}
