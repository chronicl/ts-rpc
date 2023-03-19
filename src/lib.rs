#![feature(type_name_of_val)]

pub use ts_rpc_core::*;
pub use ts_rpc_macros::ts_export;
// Todo: exporting self so TS derive macro works as long as we `use ts_rpc::ts_rs;`.
// Make this better, maybe move ts_rs fully into this crate.
pub use ts_rs::{self, TS};
