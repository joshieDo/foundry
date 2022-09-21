use super::transaction::TransactionWithMetadata;

use ethers::{
    prelude::{Http, Middleware, Provider, RetryClient, U256},
    types::transaction::eip2718::TypedTransaction,
};
use foundry_common::get_http_provider;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct ProvidersManager {
    pub inner: HashMap<String, ProviderInfo>,
}

/// Holds related metadata to each provider RPC.
pub struct ProviderInfo {
    pub provider: Arc<Provider<RetryClient<Http>>>,
    pub chain: u64,
    pub gas_price: Option<U256>,
    pub eip1559_fees: Option<(U256, U256)>,
}

impl ProviderInfo {
    pub async fn new(rpc: &str, tx: &TransactionWithMetadata) -> eyre::Result<ProviderInfo> {
        let provider = Arc::new(get_http_provider(rpc));
        let chain = provider.get_chainid().await?.as_u64();
        let (gas_price, eip1559_fees) = {
            match tx.typed_tx() {
                TypedTransaction::Legacy(_) | TypedTransaction::Eip2930(_) => {
                    (provider.get_gas_price().await.ok(), None)
                }
                TypedTransaction::Eip1559(_) => {
                    (None, provider.estimate_eip1559_fees(None).await.ok())
                }
            }
        };
        Ok(ProviderInfo { provider, chain, gas_price, eip1559_fees })
    }
}
