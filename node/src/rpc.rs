#![warn(missing_docs)]

use std::sync::Arc;

use contracts_parachain_runtime::{opaque::Block, AccountId, Balance, Nonce};

use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

/// 定义RPC扩展的类型别名
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// 包含完整依赖项的结构体
pub struct FullDeps<C, P> {
    /// 客户端实例，用于与区块链交互
	pub client: Arc<C>,
    /// 交易池实例，用于管理待处理的交易
	pub pool: Arc<P>,
}

/// 创建完整的RPC扩展
pub fn create_full<C, P>(
    deps: FullDeps<C, P>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    // 客户端类型需要实现的 trait
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static,
    // 客户端的API类型需要实现的trait
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: BlockBuilder<Block>,
    // 交易池类型需要实现的trait
	P: TransactionPool + Sync + Send + 'static,
{
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

    // 初始化一个空的RPC扩展模块
	let mut module = RpcExtension::new(());
    // 解构FullDeps以获取client和pool
	let FullDeps { client, pool } = deps;

    // 将System和TransactionPayment的RPC接口合并到模块中
	module.merge(System::new(client.clone(), pool).into_rpc())?;
	module.merge(TransactionPayment::new(client).into_rpc())?;
    // 返回构建好的RPC扩展模块
	Ok(module)
}