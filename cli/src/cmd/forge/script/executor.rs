use crate::{
    cmd::{needs_setup, TransactionWithMetadata},
    utils,
};

use cast::executor::inspector::DEFAULT_CREATE2_DEPLOYER;
use ethers::{
    prelude::NameOrAddress,
    solc::artifacts::CompactContractBytecode,
    types::{transaction::eip2718::TypedTransaction, Address, U256},
};
use forge::{
    executor::{builder::Backend, ExecutorBuilder},
    trace::CallTraceDecoder,
};

use std::collections::VecDeque;

use crate::cmd::forge::script::*;

impl ScriptArgs {
    /// Locally deploys and executes the contract method that will collect all broadcastable
    /// transactions.
    pub async fn execute(
        &self,
        script_config: &mut ScriptConfig,
        contract: CompactContractBytecode,
        sender: Address,
        predeploy_libraries: &[ethers::types::Bytes],
    ) -> eyre::Result<ScriptResult> {
        let CompactContractBytecode { abi, bytecode, .. } = contract;

        let abi = abi.expect("no ABI for contract");
        let bytecode = bytecode.expect("no bytecode for contract").object.into_bytes().unwrap();

        let mut runner = self.prepare_runner(script_config, sender).await;
        let (address, mut result) = runner.setup(
            predeploy_libraries,
            bytecode,
            needs_setup(&abi),
            script_config.sender_nonce,
            self.broadcast,
            script_config.evm_opts.fork_url.is_none(),
        )?;

        let (func, calldata) = self.get_method_and_calldata(&abi)?;
        script_config.called_function = Some(func);

        let script_result = runner.script(address, calldata)?;

        result.success &= script_result.success;
        result.gas = script_result.gas;
        result.logs.extend(script_result.logs);
        result.traces.extend(script_result.traces);
        result.debug = script_result.debug;
        result.labeled_addresses.extend(script_result.labeled_addresses);
        result.returned = script_result.returned;

        match (&mut result.transactions, script_result.transactions) {
            (Some(txs), Some(new_txs)) => {
                txs.extend(new_txs);
            }
            (None, Some(new_txs)) => {
                result.transactions = Some(new_txs);
            }
            _ => {}
        }

        Ok(result)
    }

    /// Executes a list of transactions locally and persists their state. Returns the transactions
    /// and any CREATE2 contract addresses created.
    pub async fn execute_transactions(
        &self,
        transactions: VecDeque<TypedTransaction>,
        script_config: &ScriptConfig,
        decoder: &mut CallTraceDecoder,
        contracts: &BTreeMap<ArtifactId, (Abi, Vec<u8>)>,
    ) -> eyre::Result<(VecDeque<TransactionWithMetadata>, Vec<Address>)> {
        let mut runner = self.prepare_runner(script_config, script_config.evm_opts.sender).await;

        let mut failed = false;
        let mut sum_gas = 0;
        let mut create2_contracts = vec![];
        // let mut metadata_list = vec![];

        if script_config.evm_opts.verbosity > 3 {
            println!("==========================");
            println!("Simulated On-chain Traces:\n");
        }

        let address_to_abi: BTreeMap<Address, (String, &Abi)> = decoder
            .contracts
            .iter()
            .filter_map(|(addr, contract_name)| {
                if let Some((_, (abi, _))) =
                    contracts.iter().find(|(artifact, _)| artifact.name == *contract_name)
                {
                    return Some((*addr, (contract_name.clone(), abi)))
                }
                None
            })
            .collect();

        let final_txs: VecDeque<TransactionWithMetadata> = transactions
            .into_iter()
            .map(|tx| match tx {
                TypedTransaction::Legacy(mut tx) => {
                    let mut result = runner
                        .simulate(
                            tx.from.expect(
                                "Transaction doesn't have a `from` address at execution time",
                            ),
                            tx.to.clone(),
                            tx.data.clone(),
                            tx.value,
                        )
                        .expect("Internal EVM error");

                    // We store the CREATE2 address, since it's hard to get it otherwise
                    if let Some(NameOrAddress::Address(to)) = tx.to {
                        if to == DEFAULT_CREATE2_DEPLOYER {
                            let address = Address::from_slice(&result.returned);
                            create2_contracts.push(address);
                        }
                    }

                    // We inflate the gas used by the transaction by x1.3 since the estimation
                    // might be off
                    tx.gas = Some(U256::from(result.gas * 13 / 10));

                    // final_txs.push_back(
                    //     ,
                    // );

                    sum_gas += result.gas;
                    if !result.success {
                        failed = true;
                    }

                    if script_config.evm_opts.verbosity > 3 {
                        for (_kind, trace) in &mut result.traces {
                            decoder.decode(trace);
                            println!("{}", trace);
                        }
                    }

                    TransactionWithMetadata::new(tx.into(), &result, &address_to_abi).unwrap()
                }
                _ => unreachable!(),
            })
            .collect();

        if failed {
            Err(eyre::Report::msg("Simulated execution failed"))
        } else {
            Ok((final_txs, create2_contracts))
        }
    }

    async fn prepare_runner(
        &self,
        script_config: &ScriptConfig,
        sender: Address,
    ) -> Runner<Backend> {
        let env = script_config.evm_opts.evm_env().await;

        // the db backend that serves all the data
        let db = Backend::new(
            utils::get_fork(&script_config.evm_opts, &script_config.config.rpc_storage_caching),
            &env,
        )
        .await;

        let mut builder = ExecutorBuilder::new()
            .with_cheatcodes(script_config.evm_opts.ffi)
            .with_config(env)
            .with_spec(crate::utils::evm_spec(&script_config.config.evm_version))
            .with_gas_limit(script_config.evm_opts.gas_limit());

        if script_config.evm_opts.verbosity >= 3 {
            builder = builder.with_tracing();
        }

        if self.debug {
            builder = builder.with_tracing().with_debugger();
        }

        Runner::new(builder.build(db), script_config.evm_opts.initial_balance, sender)
    }
}
