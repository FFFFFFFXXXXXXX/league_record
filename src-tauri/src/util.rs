use std::cmp::Ordering;
use std::path::Path;

use anyhow::Result;

#[macro_export]
macro_rules! cancellable {
    ($function:expr, $cancel_token:expr, Option) => {
        select! {
            option = $function => option,
            _ = $cancel_token.cancelled() => None
        }
    };
    ($function:expr, $cancel_token:expr, Result) => {
        select! {
            result = $function => result.map_err(|e| anyhow::anyhow!("{e}")),
            _ = $cancel_token.cancelled() => Err(anyhow::anyhow!("cancelled"))
        }
    };
    ($function:expr, $cancel_token:expr, ()) => {
        select! {
            _ = $function => false,
            _ = $cancel_token.cancelled() => true
        }
    };
}

pub fn compare_time(a: &Path, b: &Path) -> Result<Ordering> {
    let a_time = a.metadata()?.created()?;
    let b_time = b.metadata()?.created()?;
    Ok(a_time.cmp(&b_time).reverse())
}
