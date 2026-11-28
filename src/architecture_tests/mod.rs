//! Architecture tests: the conventions the repository writes down (CLAUDE.md,
//! doc/, the rust-architect skill) as rules `cargo test`
//! checks. One file per rule, holding its red and green rule tests; the code
//! that already broke a rule when it arrived is its baseline,
//! `baselines/<rule>.txt`, which only shrinks. To migrate a unit of a
//! baseline: fix the code, then delete its line.

mod baseline;
mod source;

mod acceptance_test_module_layout;
mod acceptance_tests_assert_whole_values;
mod acceptance_tests_skip_raw_sql;
mod adapter_folder_layout;
mod adapter_tests_live_in_sibling_file;
mod clock_is_a_port;
mod contract_tests_are_ignored;
mod domain_errors_live_in_errors_rs;
mod domain_is_call_agnostic;
mod domain_service_layout;
mod domain_services_are_flat;
mod dtos_have_no_tests;
mod fn_ordering;
mod handler_tests_skip_the_router;
mod handlers_extract_only_their_services;
mod handlers_return_typed_results;
mod persistence_is_not_faked;
mod port_traits_live_in_infrastructure;
mod repository_files_are_named_after_their_port;
mod repository_ports_return_anyhow_results;
mod response_dtos_are_flat;
mod test_naming;
