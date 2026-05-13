//! Rayon thread pool sized to half the available cores so parallel work doesn't starve the Tauri runtime.

use std::sync::LazyLock;

fn thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1))
        .unwrap_or(2)
}

/// Sets rayon's global pool so `par_iter` calls inside retoc and repak run on
/// the same shared thread count instead of spinning up their own default pool.
pub(crate) fn init_global_pool() {
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count())
        .build_global();
}

#[allow(clippy::expect_used)]
pub(crate) static POOL: LazyLock<rayon::ThreadPool> = LazyLock::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count())
        .build()
        .expect("failed to build scoped rayon pool")
});
