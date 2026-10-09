//! Media transcoding feature slice.

#[cfg(target_arch = "wasm32")]
pub(crate) mod api;
#[cfg(any(target_arch = "wasm32", test))]
mod association;
#[cfg(target_arch = "wasm32")]
mod association_list_view;
#[cfg(target_arch = "wasm32")]
mod association_view;
#[cfg(any(target_arch = "wasm32", test))]
mod job_action;
#[cfg(target_arch = "wasm32")]
mod job_action_view;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod logic;
#[cfg(any(target_arch = "wasm32", test))]
mod manual_discovery;
#[cfg(target_arch = "wasm32")]
mod manual_discovery_view;
#[cfg(target_arch = "wasm32")]
mod policy_output_view;
#[cfg(any(target_arch = "wasm32", test))]
mod profile_authoring;
#[cfg(target_arch = "wasm32")]
mod profile_authoring_fields;
#[cfg(any(target_arch = "wasm32", test))]
mod profile_list;
#[cfg(target_arch = "wasm32")]
mod profile_list_view;
#[cfg(any(target_arch = "wasm32", test))]
mod profile_roots;
#[cfg(target_arch = "wasm32")]
mod profile_roots_view;
#[cfg(target_arch = "wasm32")]
mod root_catalog_api;
#[cfg(target_arch = "wasm32")]
mod root_catalog_view;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod root_readiness;
#[cfg(target_arch = "wasm32")]
mod root_readiness_view;
#[cfg(any(target_arch = "wasm32", test))]
mod schedule;
#[cfg(target_arch = "wasm32")]
mod schedule_view;
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) mod state;
#[cfg(target_arch = "wasm32")]
pub(crate) mod view;
