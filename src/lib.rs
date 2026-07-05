// src/lib.rs
//
// Library crate — exposes all modules for integration tests and external use.
// main.rs delegates to axum_api::main() for the actual server startup.

pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod schemas;
pub mod services;
pub mod state;
