use pyo3::prelude::*;
use std::fs::read_to_string;
use std::path::Path;

/// This needs to be in a spawn_blocking or something cause this gonna block like hellll
pub fn launch_scripting_engine(script: impl AsRef<Path>) -> PyResult<()> {
    let name = script
        .as_ref()
        .file_name()
        .map(|x| x.to_str())
        .flatten()
        .unwrap_or_default();
    let script_contents = read_to_string(&script)?;

    Python::with_gil(move |py| -> PyResult<()> {
        let module = PyModule::from_code(py, &script_contents, name, "murk_script")?;

        Ok(())
    })
}
