# SpaceKit Contract SDK

Minimal `no_std` SDK for building SpaceKit WASM smart contracts. Contracts compile to `wasm32-unknown-unknown` and run on the [SpaceKit VM](https://github.com/spacekit-xyz/spacekit-js) (browser and compute node).

**Repository:** [github.com/spacekit-xyz/spacekit-contract-sdk](https://github.com/spacekit-xyz/spacekit-contract-sdk)

---
Note: This SDK is a work in progress and is not yet published to crates.io. It is only available from the SpaceKit GitHub repository. Pardon our dust.
---

## What it provides

- **`SpacekitContract`** trait and **`spacekit_contract!`** macro — export `main(input_ptr, input_len)` and `get_result(dest_ptr, max_len)` for the host.
- **Events** — `emit_event_bytes` / `emit_event_signature` (host: `env.emit_event`).
- **DID** — `get_caller_did_string`, `verify_did_string`.
- **LLM** — `llm_call`, `llm_get_status`, `llm_status` constants; host module `spacekit_llm`.
- **Storage** — `storage_set`, `storage_get`, string helpers; host module `spacekit_storage`.
- **Errors** — `ContractError` with `Failed`, `InvalidInput`, `StorageError`, `HostError`, `LlmError`, `LlmNotReady`.
- **Prelude** — `prelude::env` (caller, emit, stubs for block_timestamp / cross-contract call).

---

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
spacekit-contract-sdk = { git = "https://github.com/spacekit-xyz/spacekit-contract-sdk" }
# or from crates.io when published:
# spacekit-contract-sdk = "0.1"
```

Contract example:

```rust
#![no_std]
extern crate alloc;

use alloc::vec::Vec;
use spacekit_contract_sdk::{SpacekitContract, ContractError, spacekit_contract};

struct Example;

impl SpacekitContract for Example {
    type Error = ContractError;
    fn init() -> Self { Self }
    fn handle(&mut self, input: &[u8]) -> Result<Vec<u8>, ContractError> {
        Ok(input.to_vec())
    }
}

spacekit_contract!(Example);
```

Build (use `panic = "abort"` in your contract’s `[profile.release]` for `wasm32`):

```bash
cargo build --target wasm32-unknown-unknown --release
```

---

## Host ABI (contract expectations)

The VM must provide:

| Module           | Functions |
|------------------|-----------|
| **env**          | `emit_event`, `get_caller_did`, `verify_did`, `msg_value` |
| **spacekit_llm** | `llm_inference`, `llm_status` |
| **spacekit_storage** | `storage_save`, `storage_load` |

See [SpaceKitJS Technical Whitepaper](https://github.com/spacekit-xyz/spacekit-js) for the full host ABI and execution model.

---

## License

Apache-2.0. See the repository for the full [LICENSE](https://github.com/spacekit-xyz/spacekit-contract-sdk/blob/main/LICENSE) file.
