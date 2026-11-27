//! Architecture tests: the conventions the repository writes down (CLAUDE.md,
//! doc/, the rust-architect skill, openspec/config.yaml) as rules `cargo test`
//! checks. One file per rule, holding its red and green rule tests; the code
//! that already broke a rule when it arrived is its baseline,
//! `baselines/<rule>.txt`, which only shrinks. To migrate a unit of a
//! baseline: fix the code, then delete its line.

mod baseline;
mod openspec;
mod source;

mod domain_errors_live_in_errors_rs;
mod domain_is_call_agnostic;
mod port_traits_live_in_infrastructure;
mod repository_files_are_named_after_their_port;
mod repository_ports_return_anyhow_results;
