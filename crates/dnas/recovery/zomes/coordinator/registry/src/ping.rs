use hdk::prelude::*;

// Accept serde_bytes to handle bin-wrapped payloads from call_remote.
#[hdk_extern]
pub fn ping(_: serde_bytes::ByteBuf) -> ExternResult<String> {
    Ok("pong".into())
}
