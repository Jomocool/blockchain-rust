// 导入必要的模块和类型
use contracts_node_runtime::{self, opaque::Block, RuntimeApi};
use futures::FutureExt;
use sc_client_api::Backend;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use std::sync::Arc;

// 定义全文客户端类型别名
pub(crate) type FullClient = sc_service::TFullClient<
	Block,
	RuntimeApi,
	sc_executor::WasmExecutor<sp_io::SubstrateHostFunctions>,
>;
// 定义全文后端类型别名
type FullBackend = sc_service::TFullBackend<Block>;
// 定义全文选择链类型别名
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// 创建新的组件
///
/// 该函数初始化并返回节点服务的部分组件，包括客户端、后端、选择链、导入队列、交易池
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		Option<Telemetry>,
	>,
	ServiceError,
> {
	// 初始化遥测
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	// 创建WASM执行器
	let executor = sc_service::new_wasm_executor(&config.executor);

	// 初始化客户端、后端、密钥库和任务管理器
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	// 创建选择链
	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	// 启动工作线程
	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	// 创建导入队列
	let import_queue = sc_consensus_manual_seal::import_queue(
		Box::new(client.clone()),
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
	);

	// 创建交易池
	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	// 返回组件
	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (telemetry),
	})
}

/// 创建新的完整节点服务
///
/// 该函数初始化并返回一个完整的节点服务，包括网络、RPC处理、共识和任务管理
pub fn new_full<
	N: sc_network::NetworkBackend<Block, <Block as sp_runtime::traits::Block>::Hash>,
>(
	config: Configuration,
	finalize_delay_sec: u64,
) -> Result<TaskManager, ServiceError> {
	// 初始化部分组件
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: mut telemetry,
	} = new_partial(&config)?;

	// 初始化网络配置
	let net_config =
		sc_network::config::FullNetworkConfiguration::<
			Block,
			<Block as sp_runtime::traits::Block>::Hash,
			N,
		>::new(&config.network, config.prometheus_config.as_ref().map(|cfg| cfg.registry.clone()));
	let metrics = N::register_notification_metrics(config.prometheus_registry());

	// 构建网络服务
	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			net_config,
			block_announce_validator_builder: None,
			warp_sync_config: None,
			block_relay: None,
			metrics,
		})?;

	// 启动离线工作者
	if config.offchain_worker.enabled {
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-worker",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				is_validator: config.role.is_authority(),
				keystore: Some(keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				enable_http_requests: true,
				custom_extensions: |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	// 初始化Prometheus注册表和RPC扩展构建器
	let prometheus_registry = config.prometheus_registry().cloned();
	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		Box::new(move |_| {
			let deps = crate::rpc::FullDeps { client: client.clone(), pool: pool.clone() };
			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	// 启动RPC服务
	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network,
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: rpc_extensions_builder,
		backend,
		system_rpc_tx,
		tx_handler_controller,
		sync_service,
		config,
		telemetry: telemetry.as_mut(),
	})?;

	// 初始化提案工厂
	let proposer = sc_basic_authorship::ProposerFactory::new(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool.clone(),
		prometheus_registry.as_ref(),
		telemetry.as_ref().map(|x| x.handle()),
	);

	// 配置即时密封参数
	let params = sc_consensus_manual_seal::InstantSealParams {
		block_import: client.clone(),
		env: proposer,
		client: client.clone(),
		pool: transaction_pool,
		select_chain,
		consensus_data_provider: None,
		create_inherent_data_providers: move |_, ()| async move {
			Ok(sp_timestamp::InherentDataProvider::from_system_time())
		},
	};

	// 启动即时密封共识
	let authorship_future = sc_consensus_manual_seal::run_instant_seal(params);
	task_manager
		.spawn_essential_handle()
		.spawn_blocking("instant-seal", None, authorship_future);

	// 配置延迟最终确定性参数
	let delayed_finalize_params = sc_consensus_manual_seal::DelayedFinalizeParams {
		client,
		spawn_handle: task_manager.spawn_handle(),
		delay_sec: finalize_delay_sec,
	};
	// 启动延迟最终确定性服务
	task_manager.spawn_essential_handle().spawn_blocking(
		"delayed_finalize",
		None,
		sc_consensus_manual_seal::run_delayed_finalize(delayed_finalize_params),
	);

	// 启动网络服务
	network_starter.start_network();
	// 返回任务管理器
	Ok(task_manager)
}