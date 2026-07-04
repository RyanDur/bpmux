pub mod auth;
pub mod ci_log_store;
pub mod ci_watcher;
pub mod crypto;

pub mod electron_capture;
pub mod local_url;
pub mod peer;
pub mod protocol;
pub mod rest;
pub mod rpc_routes;
pub mod self_update;
pub(crate) mod state_delta;
pub mod test_routes;
pub mod test_seed;
pub mod unfurl;
pub mod webrtc;
pub mod ws;

#[cfg(test)]
mod auth_tests;
#[cfg(test)]
mod ci_log_persistence;
#[cfg(test)]
mod ci_watcher_tests;
#[cfg(test)]
mod crypto_tests;
#[cfg(test)]
mod external_mutation_tests;
#[cfg(test)]
mod local_url_formatting;
#[cfg(test)]
mod peer_tests;
#[cfg(test)]
mod protocol_tests;
#[cfg(test)]
mod rest_tests;
#[cfg(test)]
mod self_update_tests;
#[cfg(test)]
mod test_routes_mock_octocrab_tests;
#[cfg(test)]
mod test_routes_tests;
#[cfg(test)]
mod unfurl_tests;
#[cfg(test)]
mod ws_tests;
