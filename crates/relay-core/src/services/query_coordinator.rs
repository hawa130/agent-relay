use futures_util::future::{FutureExt, LocalBoxFuture, Shared};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::yield_now;

const MAX_NETWORK_QUERY_CONCURRENCY: usize = 32;

#[derive(Clone)]
pub struct QueryCoordinator<K, V, E> {
    inflight: Rc<RefCell<HashMap<K, Shared<LocalBoxFuture<'static, Result<V, E>>>>>>,
    semaphore: Arc<Semaphore>,
    limit: Arc<AtomicUsize>,
}

impl<K, V, E> QueryCoordinator<K, V, E>
where
    K: Clone + Eq + Hash + 'static,
    V: Clone + 'static,
    E: Clone + 'static,
{
    pub fn new(initial_limit: usize) -> Self {
        Self {
            inflight: Rc::new(RefCell::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(MAX_NETWORK_QUERY_CONCURRENCY)),
            limit: Arc::new(AtomicUsize::new(clamp_limit(initial_limit))),
        }
    }

    pub fn set_limit(&self, value: usize) {
        self.limit.store(clamp_limit(value), Ordering::Relaxed);
    }

    pub async fn run<F, Fut>(&self, key: K, operation: F) -> Result<V, E>
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = Result<V, E>> + 'static,
    {
        if let Some(existing) = self.inflight.borrow().get(&key).cloned() {
            return existing.await;
        }

        let coordinator = self.clone();
        let key_for_cleanup = key.clone();
        let shared = async move {
            let _permit = coordinator.acquire_permit().await;
            let result = operation().await;
            coordinator.inflight.borrow_mut().remove(&key_for_cleanup);
            result
        }
        .boxed_local()
        .shared();

        self.inflight
            .borrow_mut()
            .insert(key.clone(), shared.clone());
        shared.await
    }

    async fn acquire_permit(&self) -> OwnedSemaphorePermit {
        loop {
            let permit = self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("query coordinator semaphore closed");
            let in_flight = MAX_NETWORK_QUERY_CONCURRENCY - self.semaphore.available_permits();
            if in_flight <= self.limit.load(Ordering::Relaxed) {
                return permit;
            }
            drop(permit);
            yield_now().await;
        }
    }
}

fn clamp_limit(value: usize) -> usize {
    value.clamp(1, MAX_NETWORK_QUERY_CONCURRENCY)
}
