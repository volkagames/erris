use std::sync::{Arc, OnceLock};
use tracing::Subscriber;

static GLOBAL_SUBSCRIBER: OnceLock<Arc<dyn Subscriber + Send + Sync + 'static>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct GlobalSubscriberAlreadyExist;

impl std::error::Error for GlobalSubscriberAlreadyExist {}

impl std::fmt::Display for GlobalSubscriberAlreadyExist {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Oh no, global subscriber already exist!")
    }
}

pub fn set_global_subscriber(
    subscriber: Arc<dyn Subscriber + Send + Sync + 'static>,
) -> Result<(), GlobalSubscriberAlreadyExist> {
    GLOBAL_SUBSCRIBER
        .set(subscriber.clone())
        .map_err(|_| GlobalSubscriberAlreadyExist)
}

#[inline]
pub fn get_global_subscriber() -> Option<Arc<dyn Subscriber + Send + Sync + 'static>> {
    GLOBAL_SUBSCRIBER.get().cloned()
}
