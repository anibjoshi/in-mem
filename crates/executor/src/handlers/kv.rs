//! KV command handlers.
//!
//! This module implements handlers for the 4 MVP KV commands by dispatching
//! directly to engine primitives via `bridge::Primitives`.

use std::sync::Arc;

use strata_core::Value;

use crate::bridge::{extract_version, to_core_run_id, validate_key, Primitives};
use crate::convert::convert_result;
use crate::types::RunId;
use crate::{Output, Result};

// =============================================================================
// MVP Handlers (4 commands)
// =============================================================================

/// Handle KvPut command.
pub fn kv_put(
    p: &Arc<Primitives>,
    run: RunId,
    key: String,
    value: Value,
) -> Result<Output> {
    let run_id = to_core_run_id(&run)?;
    convert_result(validate_key(&key))?;
    let version = convert_result(p.kv.put(&run_id, &key, value))?;
    Ok(Output::Version(extract_version(&version)))
}

/// Handle KvGet command.
pub fn kv_get(p: &Arc<Primitives>, run: RunId, key: String) -> Result<Output> {
    let run_id = to_core_run_id(&run)?;
    convert_result(validate_key(&key))?;
    let result = convert_result(p.kv.get(&run_id, &key))?;
    Ok(Output::Maybe(result))
}

/// Handle KvDelete command.
pub fn kv_delete(p: &Arc<Primitives>, run: RunId, key: String) -> Result<Output> {
    let run_id = to_core_run_id(&run)?;
    convert_result(validate_key(&key))?;
    let existed = convert_result(p.kv.delete(&run_id, &key))?;
    Ok(Output::Bool(existed))
}

/// Handle KvList command.
pub fn kv_list(
    p: &Arc<Primitives>,
    run: RunId,
    prefix: Option<String>,
) -> Result<Output> {
    let run_id = to_core_run_id(&run)?;
    if let Some(ref pfx) = prefix {
        if !pfx.is_empty() {
            convert_result(validate_key(pfx))?;
        }
    }
    let keys = convert_result(p.kv.list(&run_id, prefix.as_deref()))?;
    Ok(Output::Keys(keys))
}
