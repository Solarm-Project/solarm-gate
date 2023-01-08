use std::{fs::File, path::Path};

use miette::{IntoDiagnostic, Result, WrapErr};

pub fn get_environment<P: AsRef<Path>>(base_path: P) -> Result<Vec<(String, String)>> {
    let env_file = File::open(base_path.as_ref().join("env.json"))
        .into_diagnostic()
        .wrap_err(format!(
            "openining {}/env.json",
            std::env::current_dir().into_diagnostic()?.display()
        ))?;
    let env: Vec<(String, String)> = serde_json::from_reader(env_file)
        .into_diagnostic()
        .wrap_err("deserializing env.json")?;
    Ok(env)
}
