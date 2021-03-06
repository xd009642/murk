use crate::request::*;
use crate::scripting::*;
use crate::spec::*;
use crate::summary::*;
use bytes::{Buf, BytesMut};
use futures::stream::{FuturesUnordered, StreamExt};
use humantime::Duration;
use hyper::body::HttpBody;
use hyper::Client;
use quanta::Clock;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration as StdDuration;
pub use structopt::StructOpt;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

pub mod request;
pub mod scripting;
pub mod spec;
pub mod summary;

#[derive(Clone, Debug, StructOpt)]
pub struct Opt {
    /// Server endpoint to test. With no config present murk will just spam HTTP GET requests to
    /// this address.
    #[structopt(name = "url")]
    endpoint: String,
    /// Number of jobs (worker threads) to use in the scheduler
    #[structopt(short = "j", long = "n-jobs")]
    jobs: Option<usize>,
    /// Number of HTTP connections to use concurrently
    #[structopt(short = "c", long = "connections")]
    connections: Option<usize>,
    /// Timeout for a request. If a request takes longer than this to respond it will be cancelled
    #[structopt(short = "t", long = "timeout")]
    timeout: Duration,
    /// Duration to run the loadtest for
    #[structopt(short = "d", long = "duration")]
    duration: Duration,
    /// Path to a configuration file
    #[structopt(long = "config")]
    config: Option<PathBuf>,
    /// Points to a script to run. See non-existing documentation for more details.
    #[structopt(long = "script")]
    script: Option<PathBuf>,
    /// Ramp up through sequences of concurrent connections. Will essentially load test at each
    /// level for the time collecting the results. So equivalent to doing multiple runs with
    /// different options for `--connections`
    #[structopt(long = "ramp")]
    ramp: Option<Vec<usize>>,
}

impl Opt {
    pub fn connections(&self) -> Vec<usize> {
        match (&self.connections, &self.ramp) {
            (Some(c), _) => vec![*c],
            (_, Some(c)) => c.clone(),
            _ => vec![500],
        }
    }

    pub fn jobs(&self) -> usize {
        self.jobs.unwrap_or_else(num_cpus::get)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunError {
    ChannelClosed,
}

async fn run_user(
    tx: mpsc::UnboundedSender<RequestStats>,
    store: Arc<RequestStore>,
    opt: Arc<Opt>,
    connections: usize,
) -> Result<(), RunError> {
    let requests = store.get_requests(store.len());
    let clock = Clock::new();
    let client = Client::new();
    let timeout_dur = *opt.timeout;
    let delay = sleep(*opt.duration);
    tokio::pin!(delay);
    for req in requests.iter().cycle() {
        let start = clock.now();
        tokio::select! {
            biased;
            res = timeout(timeout_dur, client.request(req.request())) => {
                match res {
                    Ok(Ok(mut s)) => {
                        let mut bytes_read = 0;
                        let mut buf = BytesMut::new();
                        while let Some(Ok(body)) = s.body_mut().data().await {
                            bytes_read += body.len();
                            buf.extend_from_slice(body.chunk());
                        }
                        let end = clock.now();
                        let request_time = Some(end.duration_since(start));
                        tx.send(RequestStats {
                            status: Some(s.status()),
                            request_time,
                            timeout: false,
                            body: Some(buf.freeze()),
                            bytes_read: Some(bytes_read),
                            bytes_written: Some(req.body_len()),
                            connections,
                        }).map_err(|_| RunError::ChannelClosed)?;
                    },
                    Ok(Err(_)) => {
                        tx.send(RequestStats {
                            status: None,
                            request_time: None,
                            timeout: false,
                            body: None,
                            bytes_read: None,
                            bytes_written: None,
                            connections,
                        }).map_err(|_| RunError::ChannelClosed)?;
                    },
                    Err(_) => {
                        tx.send(RequestStats {
                            status: None,
                            request_time: None,
                            timeout: true,
                            body: None,
                            bytes_read: None,
                            bytes_written: None,
                            connections,
                        }).map_err(|_| RunError::ChannelClosed)?;
                    },
                }
            }
            _ = &mut delay => {
                break;
            }
        }
    }
    Ok(())
}

pub async fn stats_collection(
    mut rx: mpsc::UnboundedReceiver<RequestStats>,
    script_channel: Option<flume::Sender<RequestStats>>,
    opt: Arc<Opt>,
) -> Summary {
    let mut summary = Summary::new(*opt.timeout);
    while let Some(stat) = rx.recv().await {
        if let Some(script) = script_channel.as_ref() {
            let _ = script.send_async(stat.clone()).await;
        }
        summary += stat;
    }
    summary
}

fn get_request_store(opt: Arc<Opt>) -> RequestStore {
    if let Some(conf) = &opt.config {
        let config = fs::read_to_string(&conf).unwrap();
        let spec = match serde_yaml::from_str::<Specification>(&config) {
            Ok(s) => s,
            Err(e) => match serde_json::from_str::<Specification>(&config) {
                Ok(s) => s,
                Err(e2) => {
                    println!("yaml error: {}", e);
                    println!("json error: {}", e2);
                    panic!("Neither valid toml or json");
                }
            },
        };
        RequestStore::create_from_spec(opt.endpoint.clone(), &spec)
    } else {
        let req = RequestBuilder::try_from(opt.endpoint.clone()).unwrap();
        RequestStore {
            requests: vec![req],
            weights: vec![1.0],
        }
    }
}

pub async fn run_loadtest(opt: Arc<Opt>) {
    let req_opt = opt.clone();

    let script_engine = if let Some(script) = opt.script.clone() {
        ScriptingContext::load(script)
    } else {
        ScriptingContext::empty()
    };
    let requests = tokio::task::spawn_blocking(move || Arc::new(get_request_store(req_opt)))
        .await
        .unwrap();
    println!("Collected {} requests. Running load test", requests.len());
    for connections in &opt.connections() {
        println!("Testing for {} concurrent connections", connections);
        let (tx, rx) = mpsc::unbounded_channel();
        let stats = tokio::task::spawn(stats_collection(
            rx,
            script_engine.response_sender(),
            opt.clone(),
        ));
        let mut jobs = FuturesUnordered::new();

        for _ in 0..*connections {
            jobs.push(tokio::task::spawn(run_user(
                tx.clone(),
                requests.clone(),
                opt.clone(),
                *connections,
            )));
        }
        while let Some(j) = jobs.next().await {
            // Closing down jobs
            if j.is_err() {
                eprintln!("Job failure, channel closed");
            }
        }
        std::mem::drop(tx);
        let summary = stats.await.unwrap();
        println!("Request summary:\n{}", summary);
        sleep(StdDuration::from_secs(2)).await;
    }

    if script_engine.is_active() {
        let end = script_engine.finish().await;
        if let Err(e) = end {
            println!("There was an error in scripty thingy: {}", e);
        }
    }
}
