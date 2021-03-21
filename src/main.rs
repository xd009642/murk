#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;
use murk::*;
use std::sync::Arc;
use tokio::runtime;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Arc::new(Opt::from_args());
    println!("Running with options:\n\t{:?}", opts);
    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(opts.jobs())
        .build()
        .unwrap();
    rt.block_on(async move {
        run_loadtest(opts).await;
    });
    Ok(())
}
