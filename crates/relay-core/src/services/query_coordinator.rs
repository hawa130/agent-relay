use futures_util::future::{FutureExt, LocalBoxFuture, Shared};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};

const MAX_NETWORK_QUERY_CONCURRENCY: usize = 32;

type SharedQueryFuture<V, E> = Shared<LocalBoxFuture<'static, Result<V, E>>>;
type InflightQueries<K, V, E> = Rc<RefCell<HashMap<K, SharedQueryFuture<V, E>>>>;

#[derive(Clone)]
pub struct QueryCoordinator<K, V, E> {
    inflight: InflightQueries<K, V, E>,
    semaphore: Arc<Semaphore>,
    limit: Arc<AtomicUsize>,
    notify: Arc<Notify>,
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
            notify: Arc::new(Notify::new()),
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
        let existing = {
            let inflight = self.inflight.borrow();
            inflight.get(&key).cloned()
        };
        if let Some(existing) = existing {
            return existing.await;
        }

        let coordinator = self.clone();
        let key_for_cleanup = key.clone();
        let shared = async move {
            let _permit = coordinator.acquire_permit().await;
            let result = operation().await;
            coordinator.inflight.borrow_mut().remove(&key_for_cleanup);
            coordinator.notify.notify_waiters();
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
            self.notify.notified().await;
        }
    }
}

fn clamp_limit(value: usize) -> usize {
    value.clamp(1, MAX_NETWORK_QUERY_CONCURRENCY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[tokio::test]
    async fn deduplicates_same_key() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let coordinator: QueryCoordinator<String, i32, String> = QueryCoordinator::new(8);
                let call_count = Rc::new(Cell::new(0));
                let (tx, rx) = tokio::sync::oneshot::channel::<i32>();

                // First run: the operation blocks on a oneshot, keeping the key inflight
                let count1 = call_count.clone();
                let coord1 = coordinator.clone();
                let handle = tokio::task::spawn_local(async move {
                    coord1
                        .run("key".into(), move || {
                            count1.set(count1.get() + 1);
                            async move { Ok(rx.await.unwrap()) }
                        })
                        .await
                });

                // Yield to let the spawned task register the inflight entry
                tokio::task::yield_now().await;

                // Second run with the same key should deduplicate
                let count2 = call_count.clone();
                let fut2 = coordinator.run("key".into(), move || {
                    count2.set(count2.get() + 1);
                    async { Ok(99) }
                });

                // Unblock the first operation
                tx.send(42).unwrap();

                let r2 = fut2.await;
                let r1 = handle.await.unwrap();
                assert_eq!(r1.unwrap(), 42);
                assert_eq!(r2.unwrap(), 42);
                assert_eq!(call_count.get(), 1);
            })
            .await;
    }

    #[tokio::test]
    async fn different_keys_run_independently() {
        let coordinator: QueryCoordinator<String, String, String> = QueryCoordinator::new(8);

        let r1 = coordinator
            .run("a".into(), || async { Ok("alpha".to_string()) })
            .await;
        let r2 = coordinator
            .run("b".into(), || async { Ok("beta".to_string()) })
            .await;

        assert_eq!(r1.unwrap(), "alpha");
        assert_eq!(r2.unwrap(), "beta");
    }

    #[tokio::test]
    async fn finished_query_is_cleaned_up() {
        let coordinator: QueryCoordinator<String, i32, String> = QueryCoordinator::new(8);

        let _ = coordinator.run("cleanup".into(), || async { Ok(1) }).await;

        // After completion, inflight map should be empty
        assert!(coordinator.inflight.borrow().is_empty());
    }

    #[tokio::test]
    async fn propagates_error() {
        let coordinator: QueryCoordinator<String, i32, String> = QueryCoordinator::new(8);

        let result = coordinator
            .run("err".into(), || async { Err("boom".to_string()) })
            .await;

        assert_eq!(result.unwrap_err(), "boom");
    }

    #[test]
    fn clamp_limit_enforces_bounds() {
        assert_eq!(clamp_limit(0), 1);
        assert_eq!(clamp_limit(1), 1);
        assert_eq!(clamp_limit(16), 16);
        assert_eq!(clamp_limit(100), MAX_NETWORK_QUERY_CONCURRENCY);
    }
}
