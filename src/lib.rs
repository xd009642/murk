use crate::summary::*;
use futures::stream::{FuturesUnordered, StreamExt};
use humantime::Duration;
use hyper::body::HttpBody;
use hyper::{client::HttpConnector, Body, Client, Uri};
use quanta::Clock;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration as StdDuration;
pub use structopt::StructOpt;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

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
}

impl Opt {
    pub fn connections(&self) -> usize {
        self.connections.unwrap_or(500)
    }

    pub fn jobs(&self) -> usize {
        self.jobs.unwrap_or_else(num_cpus::get)
    }
}

#[derive(Debug, Clone)]
pub struct MurkSession {
    setup_fn: (),
    init_fn: (),
    deay_fn: (),
    request_fn: (),
    response_fn: (),
}

async fn run_user(tx: mpsc::UnboundedSender<RequestStats>, opt: Arc<Opt>) {
    let uri: Uri = opt.endpoint.parse().expect("Invalid URL");
    let clock = Clock::new();
    let client = Client::new();
    let timeout_dur = *opt.timeout;
    let delay = sleep(*opt.duration);
    tokio::pin!(delay);
    loop {
        let start = clock.now();
        tokio::select! {
            biased;
            res = timeout(timeout_dur, client.get(uri.clone())) => {
                match res {
                    Ok(Ok(mut s)) => {
                        let mut bytes_read = 0;
                        while let Some(Ok(body)) = s.body_mut().data().await {
                            bytes_read += body.len();
                        }
                        let end = clock.now();
                        let request_time = Some(end.duration_since(start));
                        tx.send(RequestStats {
                            status: Some(s.status()),
                            request_time,
                            timeout: false,
                            bytes_read: Some(bytes_read),
                        });
                    },
                    Ok(Err(e)) => {
                        tx.send(RequestStats {
                            status: None,
                            request_time: None,
                            timeout: false,
                            bytes_read: None,
                        });
                    },
                    Err(e) => {
                        tx.send(RequestStats {
                            status: None,
                            request_time: None,
                            timeout: true,
                            bytes_read: None,
                        });
                    },
                }
            }
            _ = &mut delay => {
                break;
            }
        }
    }
}

pub async fn stats_collection(
    mut rx: mpsc::UnboundedReceiver<RequestStats>,
    opt: Arc<Opt>,
) -> Summary {
    let mut summary = Summary::new(*opt.timeout);
    while let Some(stat) = rx.recv().await {
        summary += stat;
    }
    summary
}

pub async fn run_loadtest(opt: Arc<Opt>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let stats = tokio::task::spawn(stats_collection(rx, opt.clone()));
    let mut jobs = FuturesUnordered::new();
    for _ in 0..opt.connections() {
        jobs.push(tokio::task::spawn(run_user(tx.clone(), opt.clone())));
    }
    while let Some(j) = jobs.next().await {
        // Closing down jobs
    }
    std::mem::drop(tx);
    let summary = stats.await.unwrap();
    println!("Request summary:\n{}", summary);
}
