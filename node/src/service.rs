pub mod dev;

use std::{sync::Arc, time::Duration};

use contracts_parachain_runtime::{
	opaque::{Block, Hash},
	RuntimeApi,
};
use cumulus_client_cli::CollatorOptions;
use cumulus_client_collator::service::CollatorService;
use cumulus_client_consensus_aura::collators::lookahead::{self as aura, Params as AuraParams};
use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use cumulus_client_consensus_proposer::Proposer;
use cumulus_client_service::{
	build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
	BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, ParachainHostFunctions,
	StartRelayChainTasksParams,
};
use cumulus_primitives_core::{
	relay_chain::{CollatorPair, ValidationCode},
	ParaId,
};
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};

use frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE;
use prometheus_endpoint::Registry;
use sc_client_api::Backend;
use sc_consensus::ImportQueue;
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::NetworkBlock;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_keystore::KeystorePtr;

type ParachainExecutor = WasmExecutor<ParachainHostFunctions>;

type ParachainClient = TFullClient<Block, RuntimeApi, ParachainExecutor>;

type ParachainBackend = TFullBackend<Block>;

type ParachainBlockImport = TParachainBlockImport<Block, Arc<ParachainClient>, ParachainBackend>;

// 定义一个名为Service的类型别名，它代表了一组部分组件，这些组件是构建 parachain 节点所必需的。
pub type Service = PartialComponents<
	ParachainClient,
	ParachainBackend,
	(),
	sc_consensus::DefaultImportQueue<Block>,
	sc_transaction_pool::FullPool<Block, ParachainClient>,
	(ParachainBlockImport, Option<Telemetry>, Option<TelemetryWorkerHandle>),
>;

// 构建并返回一个新的部分组件集合。这个函数初始化了 parachain 节点的核心组件。
pub fn new_partial(config: &Configuration) -> Result<Service, sc_service::Error> {
	// 根据配置初始化telemetry，如果配置了telemetry端点，则创建telemetry工作器和telemetry句柄。
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

	// 根据配置确定堆页面的数量，如果没有配置，则使用默认的堆分配策略。
	let heap_pages = config
		.executor
		.default_heap_pages
		.map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static { extra_pages: h as _ });

	// 构建一个 parachain 特定的执行器，它将用于执行 runtime 代码。
	let executor = ParachainExecutor::builder()
		.with_execution_method(config.executor.wasm_method)
		.with_onchain_heap_alloc_strategy(heap_pages)
		.with_offchain_heap_alloc_strategy(heap_pages)
		.with_max_runtime_instances(config.executor.max_runtime_instances)
		.with_runtime_cache_size(config.executor.runtime_cache_size)
		.build();

	// 初始化客户端、后端、密钥库容器和任务管理器
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts_record_import::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
			true,
		)?;
	let client = Arc::new(client);

	// 获取telemetry工作器的句柄，以便稍后启动telemetry任务。
	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	// 如果配置了telemetry，则启动telemetry任务。
	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	// 创建一个全功能的交易池，它将用于管理待处理的交易。
	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	// 创建一个区块导入对象，它将用于导入新的区块到链中。
	let block_import = ParachainBlockImport::new(client.clone(), backend.clone());

	// 构建一个导入队列，它将用于处理和导入新的区块。
	let import_queue = build_import_queue(
		client.clone(),
		block_import.clone(),
		config,
		telemetry.as_ref().map(|telemetry| telemetry.handle()),
		&task_manager,
	);

	// 返回包含所有初始化组件的部分组件集合。
	Ok(PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain: (),
		other: (block_import, telemetry, telemetry_worker_handle),
	})
}

/// 构建导入队列
///
/// 该函数初始化并返回一个区块导入队列，该队列用于处理待导入的区块。
/// 它配置了用于验证 Aura 共识算法的必要检查，包括时间戳的验证。
///
/// # 参数
///
/// * `client` - 区块链客户端，用于与区块链数据交互。
/// * `block_import` - 区块导入对象，负责实际的区块导入工作。
/// * `config` - 区块链配置的引用，包含启动配置信息。
/// * `telemetry` - 可选的telemetry句柄，用于性能监控和日志记录。
/// * `task_manager` - 任务管理器的引用，用于管理异步任务。
///
/// # 返回值
///
/// 返回一个默认的区块导入队列，用于处理和验证待导入的区块。
fn build_import_queue(
	client: Arc<ParachainClient>,
	block_import: ParachainBlockImport,
	config: &Configuration,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
) -> sc_consensus::DefaultImportQueue<Block> {
	// 初始化一个完全验证的导入队列，用于检测 Aura 共识算法中的等价性违规行为
	cumulus_client_consensus_aura::equivocation_import_queue::fully_verifying_import_queue::<
		sp_consensus_aura::sr25519::AuthorityPair,
		_,
		_,
		_,
		_,
	>(
		client,
		block_import,
		// 定义一个闭包，用于在必要时提供时间戳 inherent 数据
		move |_, _| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
			Ok(timestamp)
		},
		// 提供一个任务管理器的必要句柄，用于异步任务的管理
		&task_manager.spawn_essential_handle(),
		// 提供 Prometheus 注册表，用于监控指标的收集
		config.prometheus_registry(),
		// 提供可选的telemetry句柄，用于远程监控和日志记录
		telemetry,
	)
}

#[allow(clippy::too_many_arguments)]
/// 启动共识算法Aura。
///
/// 该函数初始化并启动Aura共识算法，用于在Parachain节点中达成共识。它通过一系列参数配置来实现这一点，
/// 包括客户端、后端、区块导入、Proposer工厂、Collator服务等。
///
/// # 参数
///
/// * `client` - Parachain客户端的Arc包装，用于与客户端交互。
/// * `backend` - Parachain后端的Arc包装，用于存储和检索链数据。
/// * `block_import` - 区块导入对象，用于导入新的区块到链中。
/// * `prometheus_registry` - 可选的Prometheus注册表引用，用于监控指标。
/// * `telemetry` - 可选的Telemetry句柄，用于telemetry。
/// * `task_manager` - 任务管理器的引用，用于管理异步任务。
/// * `relay_chain_interface` - Relay链接口的Arc包装，用于与Relay链交互。
/// * `transaction_pool` - 交易池的Arc包装，用于管理待处理的交易。
/// * `keystore` - 密钥库的指针，用于存储和管理密钥。
/// * `relay_chain_slot_duration` - Relay链的插槽持续时间。
/// * `para_id` - Parachain的ID。
/// * `collator_key` - Collator的密钥对。
/// * `overseer_handle` - Overseer句柄，用于与Overseer交互。
/// * `announce_block` - 一个用于广播新区块的回调函数。
///
/// # 返回值
///
/// 返回一个Result，表示操作是否成功。如果成功，返回Ok(()); 如果失败，返回一个错误。
fn start_consensus(
	client: Arc<ParachainClient>,
	backend: Arc<ParachainBackend>,
	block_import: ParachainBlockImport,
	prometheus_registry: Option<&Registry>,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
	relay_chain_interface: Arc<dyn RelayChainInterface>,
	transaction_pool: Arc<sc_transaction_pool::FullPool<Block, ParachainClient>>,
	keystore: KeystorePtr,
	relay_chain_slot_duration: Duration,
	para_id: ParaId,
	collator_key: CollatorPair,
	overseer_handle: OverseerHandle,
	announce_block: Arc<dyn Fn(Hash, Option<Vec<u8>>) + Send + Sync>,
) -> Result<(), sc_service::Error> {
	// 创建Proposer工厂，用于生成Proposer实例。
	let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool,
		prometheus_registry,
		telemetry.clone(),
	);

	// 使用Proposer工厂创建Proposer实例。
	let proposer = Proposer::new(proposer_factory);

	// 初始化Collator服务，用于处理Collation任务。
	let collator_service = CollatorService::new(
		client.clone(),
		Arc::new(task_manager.spawn_handle()),
		announce_block,
		client.clone(),
	);

	// 配置Aura参数，用于启动Aura共识。
	let params = AuraParams {
		create_inherent_data_providers: move |_, ()| async move { Ok(()) },
		block_import,
		para_client: client.clone(),
		para_backend: backend,
		relay_client: relay_chain_interface,
		code_hash_provider: move |block_hash| {
			client.code_at(block_hash).ok().map(|c| ValidationCode::from(c).hash())
		},
		keystore,
		collator_key,
		para_id,
		overseer_handle,
		relay_chain_slot_duration,
		proposer,
		collator_service,
		authoring_duration: Duration::from_millis(2000),
		reinitialize: false,
	};

	// 启动Aura共识算法。
	let fut = aura::run::<Block, sp_consensus_aura::sr25519::AuthorityPair, _, _, _, _, _, _, _, _>(
		params,
	);
	task_manager.spawn_essential_handle().spawn("aura", None, fut);

	// 如果一切顺利，返回Ok表示成功。
	Ok(())
}

#[sc_tracing::logging::prefix_logs_with("Parachain")]
/// 启动一个 parachain 节点的异步函数。
///
/// 该函数负责初始化和启动一个 parachain 节点，包括配置准备、网络构建、任务启动等。
/// 它接受多个配置参数，并返回一个包含任务管理器和 parachain 客户端。
///
/// # 参数
///
/// * `parachain_config`: Parachain 的配置信息。
/// * `polkadot_config`: Polkadot 中继链的配置信息。
/// * `collator_options`: Collator 的配置选项。
/// * `para_id`: Parachain 的 ID。
/// * `hwbench`: 可选的硬件基准测试信息。
///
/// # 返回值
///
/// 返回包含任务管理器和 parachain 客户端。
pub async fn start_parachain_node(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	para_id: ParaId,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient>)> {
	// 准备节点配置。
	let parachain_config = prepare_node_config(parachain_config);

	// 创建新的部分组件。
	let params = new_partial(&parachain_config)?;
	let (block_import, mut telemetry, telemetry_worker_handle) = params.other;
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let net_config = sc_network::config::FullNetworkConfiguration::<
		_,
		_,
		sc_network::NetworkWorker<Block, Hash>,
	>::new(&parachain_config.network, prometheus_registry.clone());

	let client = params.client.clone();
	let backend = params.backend.clone();
	let mut task_manager = params.task_manager;

	// 构建 relay chain 接口。
	let (relay_chain_interface, collator_key) = build_relay_chain_interface(
		polkadot_config,
		&parachain_config,
		telemetry_worker_handle,
		&mut task_manager,
		collator_options.clone(),
		hwbench.clone(),
	)
	.await
	.map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

	// 检查节点角色是否为验证者。
	let validator = parachain_config.role.is_authority();
	let transaction_pool = params.transaction_pool.clone();
	let import_queue_service = params.import_queue.service();

	// 构建网络。
	let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
		build_network(BuildNetworkParams {
			parachain_config: &parachain_config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			para_id,
			spawn_handle: task_manager.spawn_handle(),
			relay_chain_interface: relay_chain_interface.clone(),
			import_queue: params.import_queue,
			sybil_resistance_level: CollatorSybilResistance::Resistant,
		})
		.await?;

	// 如果配置了 offchain worker，则启动 offchain workers runner。
	if parachain_config.offchain_worker.enabled {
		use futures::FutureExt;

		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-work",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				keystore: Some(params.keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				is_validator: parachain_config.role.is_authority(),
				enable_http_requests: false,
				custom_extensions: move |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	// 设置 RPC 构建器。
	let rpc_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |_| {
			let deps =
				crate::rpc::FullDeps { client: client.clone(), pool: transaction_pool.clone() };

			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	// 启动任务。
	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.keystore(),
		backend: backend.clone(),
		network,
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
	})?;

	// 如果提供了硬件基准测试信息，则打印并上传到 telemetry。
	if let Some(hwbench) = hwbench {
		sc_sysinfo::print_hwbench(&hwbench);
		match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench, false) {
			Err(err) if validator => {
				log::warn!(
				"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority'.",
				err
			);
			},
			_ => {},
		}

		if let Some(ref mut telemetry) = telemetry {
			let telemetry_handle = telemetry.handle();
			task_manager.spawn_handle().spawn(
				"telemetry_hwbench",
				None,
				sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
			);
		}
	}

	// 设置区块公告。
	let announce_block = {
		let sync_service = sync_service.clone();
		Arc::new(move |hash, data| sync_service.announce_block(hash, data))
	};

	// 设置 relay chain slot 时长。
	let relay_chain_slot_duration = Duration::from_secs(6);

	// 获取 overseer handle。
	let overseer_handle = relay_chain_interface
		.overseer_handle()
		.map_err(|e| sc_service::Error::Application(Box::new(e)))?;

	// 启动 relay chain 任务。
	start_relay_chain_tasks(StartRelayChainTasksParams {
		client: client.clone(),
		announce_block: announce_block.clone(),
		para_id,
		relay_chain_interface: relay_chain_interface.clone(),
		task_manager: &mut task_manager,
		da_recovery_profile: if validator {
			DARecoveryProfile::Collator
		} else {
			DARecoveryProfile::FullNode
		},
		import_queue: import_queue_service,
		relay_chain_slot_duration,
		recovery_handle: Box::new(overseer_handle.clone()),
		sync_service: sync_service.clone(),
	})?;

	// 如果节点是验证者，则启动共识。
	if validator {
		start_consensus(
			client.clone(),
			backend,
			block_import,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			relay_chain_interface,
			transaction_pool,
			params.keystore_container.keystore(),
			relay_chain_slot_duration,
			para_id,
			collator_key.expect("Command line arguments do not allow this. qed"),
			overseer_handle,
			announce_block,
		)?;
	}

	// 启动网络。
	start_network.start_network();

	// 返回任务管理器和客户端。
	Ok((task_manager, client))
}
