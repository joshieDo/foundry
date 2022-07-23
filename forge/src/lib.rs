/// Gas reports
pub mod gas_report;

/// Coverage reports
pub mod coverage;

/// The Forge test runner
mod runner;
pub use runner::ContractRunner;

/// Forge test runners for multiple contracts
mod multi_runner;
pub use multi_runner::{MultiContractRunner, MultiContractRunnerBuilder};

mod traits;
pub use traits::*;

pub mod result;

#[cfg(test)]
mod test_helpers;

/// The Forge EVM backend
pub use foundry_evm::*;

/// Metadata on how to run fuzz/invariant tests
#[derive(Debug, Clone, Copy, Default)]
pub struct TestOptions {
    /// Whether fuzz tests should be run
    pub include_fuzz_tests: bool,
    /// The number of test cases that must execute for each fuzz test
    pub fuzz_runs: u32,
    /// The maximum number of global test case rejections allowed
    /// by proptest, to be encountered during usage of `vm.assume`
    /// cheatcode.
    pub fuzz_max_local_rejects: u32,
    /// The maximum number of local test case rejections allowed
    /// by proptest, to be encountered during usage of `vm.assume`
    /// cheatcode.
    pub fuzz_max_global_rejects: u32,
    /// The number of runs that must execute for each invariant test group.
    pub invariant_runs: u32,
    /// The number of calls executed to attempt to break invariants in one run.
    pub invariant_depth: u32,
    /// Fails the invariant fuzzing if a revert occurs
    pub invariant_fail_on_revert: bool,
    /// Allows randomly overriding an external call when running invariant tests
    pub invariant_call_override: bool,
}
