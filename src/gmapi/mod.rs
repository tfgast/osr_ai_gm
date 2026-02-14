mod combat_handlers;
mod exploration_handlers;
pub mod interface;
pub mod protocol;
mod query_handlers;

use crate::persist::GameState;
use protocol::GMResponse;
use serde::Serialize;

pub(crate) fn ok_with_typed_data<T: Serialize>(
    id: &str,
    state: &GameState,
    message: String,
    payload: T,
) -> GMResponse {
    match serde_json::to_value(payload) {
        Ok(data) => GMResponse::ok_with_data(id, message, state.mode.clone(), data),
        Err(err) => GMResponse::err(
            id,
            format!("internal error: failed to serialize response: {err}"),
            state.mode.clone(),
        ),
    }
}
