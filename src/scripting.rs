use crate::summary::*;
use flume::{Receiver, Sender};
use pyo3::prelude::*;
use std::fs::read_to_string;
use std::path::Path;
use tokio::task::{spawn_blocking, JoinHandle};

#[derive(Default)]
pub struct ScriptingContext {
    response_tx: Option<Sender<RequestStats>>,
    output_rx: Option<Receiver<ScriptEvents>>,
    handle: Option<JoinHandle<PyResult<()>>>,
}

impl ScriptingContext {
    pub fn load(script: impl AsRef<Path>) -> Self {
        let (response_tx, response_rx) = flume::unbounded();
        let (output_tx, output_rx) = flume::unbounded();
        let script = script.as_ref().to_path_buf();
        let handle =
            spawn_blocking(move || launch_scripting_engine(script, response_rx, output_tx));
        Self {
            response_tx: Some(response_tx),
            output_rx: Some(output_rx),
            handle: Some(handle),
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.handle.is_some() && self.response_tx.is_some() && self.output_rx.is_some()
    }

    pub async fn finish(self) -> PyResult<()> {
        if let Some(hnd) = self.handle {
            hnd.await.unwrap()
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScriptEvents {
    RegisterHistogram {
        name: String,
        min: u64,
        max: u64,
        accuracy: Option<u8>,
    },
    UpdateHistogram {
        name: String,
        value: u64,
    },
}

/// This needs to be in a spawn_blocking or something cause this gonna block like hellll
fn launch_scripting_engine(
    script: impl AsRef<Path>,
    responses: Receiver<RequestStats>,
    outputs: Sender<ScriptEvents>,
) -> PyResult<()> {
    let name = script
        .as_ref()
        .file_name()
        .map(|x| x.to_str())
        .flatten()
        .unwrap_or_default();
    let script_contents = read_to_string(&script)?;

    Python::with_gil(move |py| -> PyResult<()> {
        let module = PyModule::from_code(py, &script_contents, name, "murk_script")?;
        if let Ok(init_stats) = module.getattr("init_stats") {
            // Call and send out whatever it is registers new histograms/collectors
            let histograms: Vec<(String, u64, u64, Option<u8>)> = init_stats
                .call0()
                .expect("Failed to call init_stats")
                .extract()
                .expect("Malformed histogram definition");

            for (name, min, max, accuracy) in histograms.iter().cloned() {
                outputs.send(ScriptEvents::RegisterHistogram {
                    name,
                    min,
                    max,
                    accuracy,
                });
            }
        }

        if let Ok(update) = module.getattr("handle_request") {}

        if let Ok(finalise) = module.getattr("teardown") {}
        Ok(())
    })
}
