use futures::stream::{FuturesUnordered, StreamExt};
use humantime::Duration;
use hyper::body::HttpBody;
use hyper::{client::HttpConnector, Body, Client, Uri};
use quanta::Clock;
use std::sync::Arc;
pub use structopt::StructOpt;
use tokio::time::{sleep, timeout};

#[derive(Clone, Debug, StructOpt)]
pub struct Opt {
    #[structopt(name = "url")]
    endpoint: String,
    #[structopt(short = "j", long = "n-jobs")]
    jobs: Option<usize>,
    #[structopt(short = "t", long = "timeout")]
    timeout: Duration,
    #[structopt(short = "d", long = "duration")]
    duration: Duration,
}

impl Opt {
    pub fn jobs(&self) -> usize {
        self.jobs.unwrap_or_else(num_cpus::get)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Summary {
    success: usize,
    failure: usize,
    timeout: usize,
}

#[derive(Debug, Clone)]
pub struct MurkSession {
    setup_fn: (),
    init_fn: (),
    deay_fn: (),
    request_fn: (),
    response_fn: (),
}

async fn run_user(opt: Arc<Opt>) -> Summary {
    let uri: Uri = opt.endpoint.parse().expect("Invalid URL");
    let mut summary = Summary::default();
    let clock = Clock::new();
    let client = Client::new();
    let start = clock.now();
    let timeout_dur = *opt.timeout;
    let delay = sleep(*opt.duration);
    tokio::pin!(delay);
    loop {
        tokio::select! {
            biased;
            res = timeout(timeout_dur, client.get(uri.clone())) => {
                match res {
                    Ok(Ok(mut s)) => {
                     //   while let Some(_body) = s.body_mut().data().await {
                      //  }
                        summary.success += 1;
                    },
                    Ok(Err(e)) => {
                        summary.failure += 1;
                    },
                    Err(e) => {
                        summary.timeout += 1;
                    },
                }
            }
            _ = &mut delay => {
                break;
            }
        }
    }
    let end = clock.now();
    let request_time = end.duration_since(start).as_secs_f32();
    println!("User ran for {}", request_time);
    summary
}

pub async fn run_loadtest(opt: Arc<Opt>) {
    let mut jobs = FuturesUnordered::new();
    for i in 0..opt.jobs() {
        jobs.push(tokio::task::spawn(run_user(opt.clone())));
    }
    let mut total_reqs = 0;
    while let Some(j) = jobs.next().await {
        println!("User finished: {:?}", j);
        total_reqs += j.unwrap().success;
    }
    println!("Total requests: {}", total_reqs);
}
