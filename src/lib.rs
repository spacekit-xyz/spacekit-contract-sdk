#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use core::ptr;

pub trait ContractErrorCode {
    fn code(self) -> i32;
}

impl ContractErrorCode for i32 {
    fn code(self) -> i32 {
        self
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum ContractError {
    Failed = -1,
    InvalidInput = -2,
    StorageError = -3,
    HostError = -4,
    LlmError = -5,
    LlmNotReady = -6,
}

impl ContractErrorCode for ContractError {
    fn code(self) -> i32 {
        self as i32
    }
}

pub trait SpacekitContract {
    type Error: ContractErrorCode + Copy;
    fn init() -> Self;
    fn handle(&mut self, input: &[u8]) -> Result<Vec<u8>, Self::Error>;
}

static mut LAST_RESULT: Option<Vec<u8>> = None;

pub fn set_result(data: Vec<u8>) {
    unsafe {
        core::ptr::addr_of_mut!(LAST_RESULT).write(Some(data));
    }
}

pub fn result_len() -> i32 {
    unsafe {
        let ptr = core::ptr::addr_of!(LAST_RESULT);
        match &*ptr {
            Some(d) => d.len() as i32,
            None => 0,
        }
    }
}

pub fn copy_result(dest_ptr: i32, max_len: i32) -> i32 {
    unsafe {
        let ptr = core::ptr::addr_of!(LAST_RESULT);
        if let Some(data) = &*ptr {
            let len = core::cmp::min(data.len(), max_len as usize);
            ptr::copy_nonoverlapping(data.as_ptr(), dest_ptr as *mut u8, len);
            return len as i32;
        }
    }
    0
}

// ═══════════════════════════════════════════════════════════════════════════════
// Core environment host functions
// ═══════════════════════════════════════════════════════════════════════════════

#[link(wasm_import_module = "env")]
extern "C" {
    fn emit_event(event_type_ptr: *const u8, event_type_len: usize, data_ptr: *const u8, data_len: usize);
    fn get_caller_did(output_ptr: *mut u8, max_len: usize) -> i32;
    fn verify_did(did_ptr: *const u8, did_len: usize) -> i32;
    fn msg_value() -> i64;
}

// ═══════════════════════════════════════════════════════════════════════════════
// LLM host functions - for AI-powered smart contracts
// ═══════════════════════════════════════════════════════════════════════════════

#[link(wasm_import_module = "spacekit_llm")]
extern "C" {
    /// Call the LLM with a prompt and write response to dest buffer.
    /// Returns: >0 = response length written, -1 = LLM not ready, -2 = inference error
    fn llm_inference(
        prompt_ptr: *const u8,
        prompt_len: usize,
        dest_ptr: *mut u8,
        max_len: usize,
        temperature: u32,  // temperature * 100 (e.g., 70 = 0.7)
        max_tokens: u32,
    ) -> i32;
    
    /// Check LLM status: 0 = not loaded, 1 = ready, 2 = loading
    fn llm_status() -> i32;
}

/// LLM status codes
pub mod llm_status {
    pub const NOT_LOADED: i32 = 0;
    pub const READY: i32 = 1;
    pub const LOADING: i32 = 2;
}

/// Check if the LLM is ready for inference
pub fn llm_is_ready() -> bool {
    unsafe { llm_status() == llm_status::READY }
}

/// Get the current LLM status
pub fn llm_get_status() -> i32 {
    unsafe { llm_status() }
}

/// Call the LLM with a prompt and return the response
/// 
/// # Arguments
/// * `prompt` - The text prompt to send to the LLM
/// * `temperature` - Temperature * 100 (e.g., 70 for 0.7)
/// * `max_tokens` - Maximum tokens to generate
/// * `max_response_len` - Maximum response buffer size
/// 
/// # Returns
/// * `Ok(String)` - The LLM response
/// * `Err(ContractError::LlmNotReady)` - LLM not loaded
/// * `Err(ContractError::LlmError)` - Inference failed
pub fn llm_call(
    prompt: &str,
    temperature: u32,
    max_tokens: u32,
    max_response_len: usize,
) -> Result<String, ContractError> {
    let mut buffer = vec![0u8; max_response_len];
    
    let result = unsafe {
        llm_inference(
            prompt.as_ptr(),
            prompt.len(),
            buffer.as_mut_ptr(),
            buffer.len(),
            temperature,
            max_tokens,
        )
    };
    
    if result == -1 {
        return Err(ContractError::LlmNotReady);
    }
    if result < 0 {
        return Err(ContractError::LlmError);
    }
    
    buffer.truncate(result as usize);
    String::from_utf8(buffer).map_err(|_| ContractError::LlmError)
}

/// Convenience function: call LLM with default parameters
/// Temperature: 0.7, Max tokens: 256, Max response: 4096 bytes
pub fn llm_chat(prompt: &str) -> Result<String, ContractError> {
    llm_call(prompt, 70, 256, 4096)
}

/// Convenience function: call LLM for analysis (lower temperature)
/// Temperature: 0.3, Max tokens: 128, Max response: 2048 bytes
pub fn llm_analyze(prompt: &str) -> Result<String, ContractError> {
    llm_call(prompt, 30, 128, 2048)
}

/// Convenience function: call LLM for summarization
/// Temperature: 0.5, Max tokens: 200, Max response: 2048 bytes
pub fn llm_summarize(prompt: &str) -> Result<String, ContractError> {
    llm_call(prompt, 50, 200, 2048)
}

pub fn emit_event_bytes(event_type: &str, data: &[u8]) {
    unsafe {
        emit_event(event_type.as_ptr(), event_type.len(), data.as_ptr(), data.len());
    }
}

pub fn emit_event_signature(signature: &str, data: &[u8]) {
    emit_event_bytes(signature, data)
}

pub fn get_caller_did_string() -> Result<alloc::string::String, ContractError> {
    let mut buffer = alloc::vec![0u8; 256];
    let len = unsafe { get_caller_did(buffer.as_mut_ptr(), buffer.len()) };
    if len <= 0 {
        return Err(ContractError::HostError);
    }
    buffer.truncate(len as usize);
    core::str::from_utf8(&buffer)
        .map(|s| s.to_string())
        .map_err(|_| ContractError::HostError)
}

pub fn verify_did_string(did: &str) -> bool {
    let res = unsafe { verify_did(did.as_ptr(), did.len()) };
    res == 1
}

pub fn msg_value_u64() -> u64 {
    unsafe { msg_value() as u64 }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Storage host functions
// ═══════════════════════════════════════════════════════════════════════════════

#[link(wasm_import_module = "spacekit_storage")]
extern "C" {
    fn storage_save(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) -> i32;
    fn storage_load(key_ptr: *const u8, key_len: usize, output_ptr: *mut u8, max_len: usize) -> i32;
}

/// Save data to persistent storage
pub fn storage_set(key: &[u8], value: &[u8]) -> Result<(), ContractError> {
    let result = unsafe {
        storage_save(key.as_ptr(), key.len(), value.as_ptr(), value.len())
    };
    if result < 0 {
        Err(ContractError::StorageError)
    } else {
        Ok(())
    }
}

/// Load data from persistent storage
/// Returns None if key doesn't exist
pub fn storage_get(key: &[u8], max_len: usize) -> Option<Vec<u8>> {
    let mut buffer = vec![0u8; max_len];
    let result = unsafe {
        storage_load(key.as_ptr(), key.len(), buffer.as_mut_ptr(), buffer.len())
    };
    if result <= 0 {
        None
    } else {
        buffer.truncate(result as usize);
        Some(buffer)
    }
}

/// Convenience: save a string to storage
pub fn storage_set_string(key: &str, value: &str) -> Result<(), ContractError> {
    storage_set(key.as_bytes(), value.as_bytes())
}

/// Convenience: load a string from storage
pub fn storage_get_string(key: &str, max_len: usize) -> Option<String> {
    storage_get(key.as_bytes(), max_len)
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Prelude module - re-exports for contract usage
// ═══════════════════════════════════════════════════════════════════════════════

pub mod prelude {
    extern crate alloc;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use alloc::vec;
    pub use alloc::format;
    pub use alloc::collections::{BTreeMap as Map, BTreeSet as Set};

    pub type Did = String;
    pub type Address = String;

    pub mod env {
        use super::{Did, String};

        /// Get the caller's DID
        pub fn caller() -> Did {
            crate::get_caller_did_string().unwrap_or_default()
        }

        /// Get block timestamp (stub - needs host function)
        pub fn block_timestamp() -> u64 {
            // TODO: Add host function for block timestamp
            0
        }

        /// Emit an event
        pub fn emit<T>(_event: &str, _data: &T) {
            // Serialization would be needed here for real use
            // For now just emit empty data
            crate::emit_event_bytes(_event, &[]);
        }

        /// Call another contract (stub - needs host function)
        pub fn call(_address: String, _method: &str, _arg: &Did) -> bool {
            // TODO: Add host function for cross-contract calls
            false
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Macros
// ═══════════════════════════════════════════════════════════════════════════════

/// Main contract macro - exports `main` and `get_result` functions
#[macro_export]
macro_rules! spacekit_contract {
    ($contract_type:ty) => {
        static mut CONTRACT_INSTANCE: core::option::Option<$contract_type> = None;

        #[no_mangle]
        pub extern "C" fn main(input_ptr: i32, input_len: i32) -> i32 {
            let input = unsafe {
                core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize)
            };

            let contract = unsafe {
                if CONTRACT_INSTANCE.is_none() {
                    CONTRACT_INSTANCE = Some(<$contract_type as $crate::SpacekitContract>::init());
                }
                CONTRACT_INSTANCE.as_mut().unwrap()
            };

            match contract.handle(input) {
                Ok(output) => {
                    $crate::set_result(output);
                    $crate::result_len()
                }
                Err(err) => err.code(),
            }
        }

        #[no_mangle]
        pub extern "C" fn get_result(dest_ptr: i32, max_len: i32) -> i32 {
            $crate::copy_result(dest_ptr, max_len)
        }
    };
}
