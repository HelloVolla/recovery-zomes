pub mod chunk;
pub mod ping;
pub mod storage_entry;
use hdk::prelude::*;
use shared_types::RemoteCallInput;

// Called the first time a zome call is made to the cell containing this zome
#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    let mut functions = HashSet::new();
    functions.insert((zome_info()?.name, "ping".into()));
    create_cap_grant(ZomeCallCapGrant {
        tag: "ping".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(functions),
    })?;
    Ok(InitCallbackResult::Pass)
}

// Test function that can be called externally
#[hdk_extern]
pub fn hello_world(name: String) -> ExternResult<String> {
    Ok(format!("Hello, {}! This is the recovery zome.", name))
}

/// Make a generic remote zome call to another agent
#[hdk_extern]
pub fn call_remote_zome(input: RemoteCallInput) -> ExternResult<ExternIO> {
    let response = call_remote(
        input.target_agent,
        ZomeName::from(input.zome_name),
        input.fn_name.into(),
        input.cap_secret,
        input.payload,
    )?;

    match response {
        ZomeCallResponse::Ok(extern_io) => Ok(extern_io),
        ZomeCallResponse::Unauthorized(_, _, _, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Unauthorized: no capability grant for this function".to_string())))
        }
        ZomeCallResponse::NetworkError(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Network error: {}", err))))
        }
        ZomeCallResponse::CountersigningSession(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Countersigning error: {}", err))))
        }
        ZomeCallResponse::AuthenticationFailed(_, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Authentication failed".to_string())))
        }
    }
}
