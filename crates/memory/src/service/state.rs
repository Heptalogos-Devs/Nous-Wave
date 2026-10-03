//! Shared state checks; domain owners retain their transitions, fences and writes.
use nous_core::{Error, Result};

pub(super) fn fence_epoch(actual: i64, expected: i64) -> Result<()> {
    if actual != expected {
        return Err(Error::Conflict("expected object epoch is stale".into()));
    }
    Ok(())
}

pub(super) fn require_transition(current: &str, from: &str) -> Result<()> {
    if current != from {
        return Err(Error::FailedPrecondition(format!(
            "object is not in {from} state"
        )));
    }
    Ok(())
}
