#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod assets_config;
mod contracts_config;
mod revive_config;

extern crate alloc;
use alloc::vec::Vec;
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
};
use frame_system::limits::{BlockLength, BlockWeights};
use polkadot_runtime_common::SlowAdjustingFeeUpdate;
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, IdentifyAccount, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature,
};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstBool, ConstU128, ConstU32, ConstU8, KeyOwnerProofSystem,
		Randomness, StorageInfo,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight,
	},
	StorageValue,
};
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::FungibleAdapter;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

/// 区块编号类型定义，使用32位无符号整数
pub type BlockNumber = u32;
/// 签名类型定义，使用多重签名
pub type Signature = MultiSignature;
/// 账户ID类型定义，通过签名验证来确定账户身份
/// 这里的类型推导依赖于签名验证机制和账户识别逻辑
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// 余额类型定义，使用128位无符号整数
pub type Balance = u128;
/// number used once
pub type Nonce = u32;
/// 哈希类型定义，使用sp_core库中的H256类型
/// H256是一个256位的哈希值
pub type Hash = sp_core::H256;

// 定义一个名为opaque的模块，用于封装一些不透明的数据结构和类型别名
pub mod opaque {
	// 导入外部模块和当前模块的父作用域中的所有内容
	use super::*;

	// 使用sp_runtime提供的OpaqueExtrinsic类型作为本模块中的UncheckedExtrinsic类型
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	// 定义一个Header类型别名，使用generic::Header，并指定BlockNumber和BlakeTwo256作为参数
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

	// 定义一个Block类型别名，使用generic::Block，并指定Header和UncheckedExtrinsic作为参数
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;

	// 定义一个BlockId类型别名，使用generic::BlockId，并指定Block作为参数
	pub type BlockId = generic::BlockId<Block>;

	// 使用impl_opaque_keys宏定义一个SessionKeys结构体，用于管理会话密钥
	impl_opaque_keys! {
		// SessionKeys结构体定义，可以根据需要添加具体的密钥类型
		pub struct SessionKeys {}
	}
}

// 运行时版本定义
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	// 设置运行时规范的名称
	spec_name: create_runtime_str!("contracts-node"),
	// 设置实现的名称
	impl_name: create_runtime_str!("contracts-node"),
	// 设置作者版本，用于标识运行时的作者
	authoring_version: 1,
	// 设置规范版本，用于标识运行时规范的版本
	spec_version: 100,
	// 设置实现版本，用于标识当前运行时实现的版本
	impl_version: 1,
	// 设置运行时API版本，用于标识运行时提供的API版本
	apis: RUNTIME_API_VERSIONS,
	// 设置事务版本，用于标识事务格式的版本
	transaction_version: 1,
	// 设置状态版本，用于标识状态格式的版本
	state_version: 1,
};

// 当启用"std"功能时，提供本地版本信息
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

// 定义常规调度比例，用于配置系统中常规调度任务所占的最大比例
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

// 定义初始化时平均占用的比例，用于配置初始化任务占用的最大比例
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

// 定义最大块权重，限制了处理交易和区块初始化时可消耗的最大资源
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

// 配置合约模块的调试输出级别
const CONTRACTS_DEBUG_OUTPUT: pallet_contracts::DebugInfo =
	pallet_contracts::DebugInfo::UnsafeDebug;

// 配置合约模块的事件收集策略
const CONTRACTS_EVENTS: pallet_contracts::CollectEvents =
	pallet_contracts::CollectEvents::UnsafeCollect;

// 配置复兴模块的调试输出级别
const REVIVE_DEBUG_OUTPUT: pallet_revive::DebugInfo = pallet_revive::DebugInfo::UnsafeDebug;

// 配置复兴模块的事件收集策略
const REVIVE_EVENTS: pallet_revive::CollectEvents = pallet_revive::CollectEvents::UnsafeCollect;

// 定义货币的最小单位，用于表示账户余额等值
const MILLIUNIT: Balance = 1_000_000_000;

// 定义账户存在的最小存款要求，以防止账户余额过低导致的问题
pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

// 实现 pallet_insecure_randomness_collective_flip 的配置
impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	// 定义区块哈希计数常量，用于指定区块链中保存的区块哈希数量
	pub const BlockHashCount: BlockNumber = 2400;

	// 定义运行时版本常量，用于标识当前运行时的版本信息
	pub const Version: RuntimeVersion = VERSION;

	// 配置区块长度限制，包括正常事务比例
	// 这里设置了最大区块长度，并保留了正常事务的比例
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);

	// 配置区块权重，以确定不同类别的事务在区块中所占的权重
	// 这里通过构建器模式设置基础块权重、事务基础权重，并区分正常和操作类事务设置最大总权重
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();

	// SS58前缀，用于标识Substrate网络中的地址格式
	pub const SS58Prefix: u8 = 42;
}

// 通过 derive_impl 宏来实现 frame_system::Config trait，这是 Substrate 框架中的一个标准配置
// 该实现是针对一个运行时（Runtime）的配置，它定义了系统模块如何在当前区块链运行时环境中工作
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	// 定义了运行时使用的区块类型
	type Block = Block;
	// 定义了区块权重的配置类型，用于管理交易的权重
	type BlockWeights = RuntimeBlockWeights;
	// 定义了区块长度的配置类型，用于管理交易的长度
	type BlockLength = RuntimeBlockLength;
	// 定义了账户ID的类型
	type AccountId = AccountId;
	// 定义了随机数（Nonce）的类型，通常用于交易计数
	type Nonce = Nonce;
	// 定义了哈希值的类型
	type Hash = Hash;
	// 定义了区块哈希计数的类型，表示区块链保存的哈希值数量
	type BlockHashCount = BlockHashCount;
	// 定义了数据库权重的类型，用于评估数据库操作的权重
	type DbWeight = RocksDbWeight;
	// 定义了版本的类型
	type Version = Version;
	// 定义了账户数据的类型，这里使用的是 balances 模块的账户数据类型
	type AccountData = pallet_balances::AccountData<Balance>;
	// 定义了 SS58 前缀的类型，SS58 是一种编码方案，用于编码区块链网络中的账户地址
	type SS58Prefix = SS58Prefix;
	// 定义了最大消费者（Consumers）数量的类型，这里设置了一个常量值 16
	// 在 Substrate 中，消费者是指依赖于某个账户存在的人或模块
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

// pallet_authorship 配置实现
// 定义常量UncleGenerations，表示叔块的最大生成深度，这里设置为0
// 这个参数用于区块链中的叔块奖励机制，设置为0意味着不考虑叔块
parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

// 实现pallet_authorship::Config trait for Runtime，配置区块的作者信息处理
impl pallet_authorship::Config for Runtime {
	// FindAuthor类型为空，表示不使用任何算法来查找区块作者
	// 这通常在不需要追踪区块作者或使用其他机制确定作者时设置
	type FindAuthor = ();

	// EventHandler类型为空，表示没有特定的事件处理逻辑
	// 这意味着区块作者信息的处理将遵循默认行为，而不是自定义行为
	type EventHandler = ();
}

// 定义 pallet_timestamp 的参数类型
parameter_types! {
	// 设置最小周期为5个时间单位，以防止时间戳设置得太快
	pub const MinimumPeriod: u64 = 5;
}

// 实现 pallet_timestamp 的配置接口
impl pallet_timestamp::Config for Runtime {
	// 指定时间戳的类型为 u64
	type Moment = u64;
	// 设置时间戳时执行的操作，这里为空
	type OnTimestampSet = ();
	// 引用最小周期常量
	type MinimumPeriod = MinimumPeriod;
	// 权重信息类型，这里为空
	type WeightInfo = ();
}

// pallet_balances 配置实现
// 定义常量 MaxLocks，表示每个账户上最多可以有的锁定余额数量
pub const MaxLocks: u32 = 50;
// 定义常量 MaxReserves，表示每个账户上最多可以有的预留余额数量
pub const MaxReserves: u32 = 50;

// 实现 pallet_balances 模块的配置接口
impl pallet_balances::Config for Runtime {
	// 指定最大锁定数量类型
	type MaxLocks = MaxLocks;
	// 指定最大预留数量类型
	type MaxReserves = MaxReserves;
	// 定义预留标识符的类型，用于区分不同的预留
	type ReserveIdentifier = [u8; 8];
	// 定义余额的类型
	type Balance = Balance;
	// 定义事件的类型，用于在模块中生成事件
	type RuntimeEvent = RuntimeEvent;
	// 定义尘埃移除的类型，这里留空表示不实现尘埃移除逻辑
	type DustRemoval = ();
	// 定义存在性存款的金额
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	// 定义账户存储的类型，这里使用 System 模块提供的账户存储
	type AccountStore = System;
	// 定义权重信息的类型，用于计算模块中各种操作的权重
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	// 定义冻结标识符的类型，这里留空表示不实现冻结逻辑
	type FreezeIdentifier = ();
	// 定义最大冻结数量的类型，这里留空表示不实现冻结逻辑
	type MaxFreezes = ();
	// 定义持有原因的类型，用于在运行时标识持有资金的原因
	type RuntimeHoldReason = RuntimeHoldReason;
	// 定义冻结原因的类型，用于在运行时标识冻结资金的原因
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

// pallet_transaction_payment 配置实现
// 为 Runtime 实现 pallet_transaction_payment::Config trait，以配置交易费用相关功能
impl pallet_transaction_payment::Config for Runtime {
	// 指定 RuntimeEvent 类型作为事件类型
	type RuntimeEvent = RuntimeEvent;
	// 使用 FungibleAdapter 作为交易费用的处理逻辑，关联 Balances 模块
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	// 设置操作费用的乘数为 5，用于调整操作性交易的费用
	type OperationalFeeMultiplier = ConstU8<5>;
	// 将权重直接转换为费用，使用 IdentityFee 实现
	type WeightToFee = IdentityFee<Balance>;
	// 将交易长度直接转换为费用，使用 IdentityFee 实现
	type LengthToFee = IdentityFee<Balance>;
	// 使用 SlowAdjustingFeeUpdate 作为费用乘数的更新逻辑
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

// pallet_sudo 配置实现
// 为 Runtime 实现 pallet_sudo::Config trait，以配置 sudo 模块
impl pallet_sudo::Config for Runtime {
	// 指定 RuntimeEvent 类型作为事件类型
	type RuntimeEvent = RuntimeEvent;
	// 指定 RuntimeCall 类型作为调用类型，允许 sudo 用户执行任意调用
	type RuntimeCall = RuntimeCall;
	// 使用 SubstrateWeight 为 sudo 操作计算权重信息
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

// pallet_utility 配置实现
// 为 Runtime 实现 pallet_utility::Config trait，以配置通用的实用功能
impl pallet_utility::Config for Runtime {
	// 指定 RuntimeEvent 类型作为运行时事件的类型
	type RuntimeEvent = RuntimeEvent;
	// 指定 RuntimeCall 类型作为运行时调用的类型
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	// 使用 SubstrateWeight 来提供权重信息
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

// 运行时定义
// 定义runtime模块，包含运行时配置和 pallets 的注册
#[frame_support::runtime]
mod runtime {
	// 定义Runtime结构体，并继承多个运行时特性
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	// 系统模块，用于基础的链操作和状态管理
	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	// 随机性模块，提供不安全的随机性来源，主要用于测试和开发环境
	#[runtime::pallet_index(1)]
	pub type RandomnessCollectiveFlip = pallet_insecure_randomness_collective_flip;

	// 实用模块，提供批处理和原子操作功能
	#[runtime::pallet_index(2)]
	pub type Utility = pallet_utility;

	// 时间戳模块，用于记录区块的时间信息
	#[runtime::pallet_index(3)]
	pub type Timestamp = pallet_timestamp;

	// 余额模块，管理账户的余额和转账逻辑
	#[runtime::pallet_index(4)]
	pub type Balances = pallet_balances;

	// 作者身份模块，用于确定区块的作者
	#[runtime::pallet_index(5)]
	pub type Authorship = pallet_authorship;

	// 交易支付模块，处理交易费用的计算和收取
	#[runtime::pallet_index(6)]
	pub type TransactionPayment = pallet_transaction_payment;

	// 超级用户模块，提供链上治理的超级用户功能
	#[runtime::pallet_index(7)]
	pub type Sudo = pallet_sudo;

	// 合约模块，支持链上智能合约的部署和执行
	#[runtime::pallet_index(8)]
	pub type Contracts = pallet_contracts;

	// 复活模块，用于管理合约的复活操作
	#[runtime::pallet_index(9)]
	pub type Revive = pallet_revive;

	// 资产模块，用于创建和管理链上资产
	#[runtime::pallet_index(10)]
	pub type Assets = pallet_assets;
}

// 定义地址类型，用于表示账户地址
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

// 定义区块头类型，包含区块的元数据
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

// 定义区块类型，包含区块头和区块体
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

// 定义签名额外数据类型，包含交易的签名信息
pub type SignedExtra = (
	// 确保消息发送者不是零地址
	frame_system::CheckNonZeroSender<Runtime>,
	// 检查交易的规格版本是否与当前运行时兼容
	frame_system::CheckSpecVersion<Runtime>,
	// 检查交易的版本是否与当前运行时版本一致
	frame_system::CheckTxVersion<Runtime>,
	// 确保交易中的创世哈希与当前链的创世哈希匹配
	frame_system::CheckGenesis<Runtime>,
	// 验证交易的时期，以防止重放攻击
	frame_system::CheckEra<Runtime>,
	// 检查并更新交易的nonce，以防止重放和确保交易顺序
	frame_system::CheckNonce<Runtime>,
	// 确保交易的权重在可接受范围内
	frame_system::CheckWeight<Runtime>,
	// 收取交易费用
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

// 定义签名负载类型，包含交易的调用信息和签名额外数据
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

// 定义未检查的外部交易类型，包含交易的地址、调用信息、签名和签名额外数据
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

// 定义执行器类型，用于执行runtime逻辑
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

// 定义事件记录类型，用于记录runtime事件
type EventRecord = frame_system::EventRecord<
	<Runtime as frame_system::Config>::RuntimeEvent,
	<Runtime as frame_system::Config>::Hash,
>;

// 实现运行时 API
impl_runtime_apis! {
	// 核心 API 实现
	// 实现 sp_api::Core trait 以提供核心区块链操作
	impl sp_api::Core<Block> for Runtime {
		// 返回运行时版本信息
		fn version() -> RuntimeVersion {
			VERSION
		}
		// 执行一个完整的区块，包括所有交易
		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}
		// 初始化区块，准备执行
		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	// 元数据 API 实现
	impl sp_api::Metadata<Block> for Runtime {
		// 提供当前运行时元数据
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
		// 提供特定版本的元数据
		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}
		// 列出所有可用的元数据版本
		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

		/// 实现BlockBuilder trait，提供区块构建功能
	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		/// 处理一个外来事务，应用其效果到当前区块中
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		/// 完成区块构建，生成区块头
		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		/// 根据内在数据生成内在事务
		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		/// 检查区块中的内在事务是否有效
		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	/// 实现TaggedTransactionQueue trait，提供交易有效性验证功能
	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		/// 验证交易的有效性，决定其是否可以加入交易池
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	/// 实现OffchainWorkerApi trait，提供离线工作器功能
	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		/// 执行离线工作器任务
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	/// 实现SessionKeys trait，管理会话密钥
	impl sp_session::SessionKeys<Block> for Runtime {
		/// 生成会话密钥
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		/// 解码会话密钥
		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

		// 实现frame_system_rpc_runtime_api::AccountNonceApi trait，提供获取账户nonce的功能
	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		/// 获取指定账户的nonce
		///
		/// # 参数
		/// * `account` - 指定的账户ID
		///
		/// # 返回值
		/// 返回指定账户的nonce值
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	// 实现pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi trait，提供交易费用相关查询功能
	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		/// 查询交易信息
		///
		/// # 参数
		/// * `uxt` - 交易的Extrinsic
		/// * `len` - 交易的长度
		///
		/// # 返回值
		/// 返回交易的RuntimeDispatchInfo信息，包含费用详情
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}

		/// 查询交易费用详情
		///
		/// # 参数
		/// * `uxt` - 交易的Extrinsic
		/// * `len` - 交易的长度
		///
		/// # 返回值
		/// 返回交易的FeeDetails信息，详细列出各项费用
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}

		/// 根据权重查询对应的费用
		///
		/// # 参数
		/// * `weight` - 交易的权重
		///
		/// # 返回值
		/// 返回与权重对应的费用
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}

		/// 根据长度查询对应的费用
		///
		/// # 参数
		/// * `length` - 交易的长度
		///
		/// # 返回值
		/// 返回与长度对应的费用
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}
	// 实现TransactionPaymentCallApi trait，提供交易支付相关的RPC接口服务
	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall> for Runtime {
		// 查询调用信息，包括费用等
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		// 查询调用的费用详情
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		// 根据权重查询对应的费用
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		// 根据长度查询对应的费用
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	// 实现ContractsApi trait，提供合约相关的RPC接口服务
	impl pallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord>
		for Runtime
	{
		// 调用合约函数
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_contracts::ContractExecResult<Balance, EventRecord> {
			// 设置gas限制，如果未提供，则使用区块的最大权重
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);

			// 调用合约功能，不包括在沙盒环境中执行
			// origin: 调用者的账户地址
			// dest: 目标合约的地址
			// value: 转移到合约的价值
			// gas_limit: 本次调用的gas上限
			// storage_deposit_limit: 存储押金的上限
			// input_data: 传递给合约的输入数据
			// CONTRACTS_DEBUG_OUTPUT: 是否启用调试输出
			// CONTRACTS_EVENTS: 是否启用事件记录
			// pallet_contracts::Determinism::Enforced: 强制执行确定性，以确保合约执行的可预测性
			Contracts::bare_call(
				origin,
				dest,
				value,
				gas_limit,
				storage_deposit_limit,
				input_data,
				CONTRACTS_DEBUG_OUTPUT,
				CONTRACTS_EVENTS,
				pallet_contracts::Determinism::Enforced,
			)
		}

		// 实例化合约
		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pallet_contracts::Code<Hash>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> pallet_contracts::ContractInstantiateResult<AccountId, Balance, EventRecord>
		{
			// 设置gas限制，如果未提供，则使用区块的最大权重
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);

			// 调用合约的裸初始化函数
			// 这个函数负责在给定的条件下实例化一个新的合约
			Contracts::bare_instantiate(
				origin,          // 起源，表示操作的发起者
				value,           // 转移到新合约的价值
				gas_limit,       // 合约执行的gas限制
				storage_deposit_limit, // 存储存款限制，限制合约可以使用的额外存储量
				code,            // 合约的字节码
				data,            // 构造函数的输入数据
				salt,            // 用于合约地址派生的盐值，确保地址唯一性
				CONTRACTS_DEBUG_OUTPUT, // 是否启用调试输出
				CONTRACTS_EVENTS,        // 是否启用事件
			)
		}

		// 上传合约代码
		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
			determinism: pallet_contracts::Determinism,
		) -> pallet_contracts::CodeUploadResult<Hash, Balance>
		{
			Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
		}

		/// 获取合约存储值
		///
		/// 此函数用于获取指定合约地址和存储键的存储值
		///
		/// # 参数
		///
		/// * `address`: 合约的账户ID，用于标识特定的合约
		/// * `key`: 一个字节向量，代表合约存储的键
		///
		/// # 返回值
		///
		/// 返回类型为 `pallet_contracts::GetStorageResult`，表示获取存储值的结果
		/// 这个结果类型封装了可能的存储值和访问状态
		///
		/// # 示例
		///
		/// ```
		/// let address = AccountId::from([0x01; 32]);
		/// let key = vec![0x00; 32];
		/// let result = get_storage(address, key);
		/// match result {
		///     Ok(Some(value)) => println!("存储值为: {:?}", value),
		///     Ok(None) => println!("未找到存储值"),
		///     Err(err) => println!("获取存储值时出错: {:?}", err),
		/// }
		/// ```
		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pallet_contracts::GetStorageResult {
			Contracts::get_storage(address, key)
		}
	}

	// 实现ReviveApi trait以提供特定的功能
	impl pallet_revive::ReviveApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord> for Runtime
	{
		/// 执行合约调用。
		///
		/// 该函数允许一个账户调用另一个账户上的合约，可以指定传递的值、gas限制、存储押金限制和输入数据。
		/// 它主要用于合约的交互和执行，其执行结果包含执行状态和事件记录。
		///
		/// # 参数
		/// - `origin`: 调用者的账户ID。
		/// - `dest`: 目标合约的账户ID。
		/// - `value`: 转移到目标合约的余额值。
		/// - `gas_limit`: gas限制，可选。如果没有提供，则使用最大块gas限制。
		/// - `storage_deposit_limit`: 存储押金限制，可选。如果没有提供，则使用最大值。
		/// - `input_data`: 传递给合约的输入数据，通常是以字节形式表示的。
		///
		/// # 返回值
		/// 返回一个包含执行结果和事件记录的结构体。
		///
		/// # 注意
		/// - 当`gas_limit`或`storage_deposit_limit`未指定时，将使用默认值。
		/// - `REVIVE_DEBUG_OUTPUT`和`REVIVE_EVENTS`是用于控制调试输出和事件记录的全局常量。
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_revive::ContractExecResult<Balance, EventRecord> {
			Revive::bare_call(
				RuntimeOrigin::signed(origin),
				dest,
				value,
				gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block),
				storage_deposit_limit.unwrap_or(u128::MAX),
				input_data,
				REVIVE_DEBUG_OUTPUT,
				REVIVE_EVENTS,
			)
		}

				/// 实例化一个新的合约。
		///
		/// 该函数负责在链上创建一个新的合约实例。它允许调用者指定初始配置，
		/// 包括代码、数据以及一些执行选项，如gas限制和存储押金限制。
		///
		/// # 参数
		/// - `origin`: 发起实例化请求的账户ID。
		/// - `value`: 随合约初始分配的余额。
		/// - `gas_limit`: 合约实例化过程中执行所允许的最大gas量，如未指定，则使用区块最大gas量。
		/// - `storage_deposit_limit`: 为合约存储支付的押金上限，如未指定，则默认最大值。
		/// - `code`: 合约的代码，可以是代码的哈希值或原始字节码。
		/// - `data`: 传递给合约构造函数的数据。
		/// - `salt`: 用于派生合约地址的盐值，相同代码和数据的不同盐值会生成不同的合约地址。
		///
		/// # 返回
		/// 返回一个包含合约实例化结果的类型，其中包括账户ID、余额和事件记录。
		///
		/// # 注意
		/// 此函数调用底层的`Revive::bare_instantiate`函数进行实际的合约实例化操作。
		/// 它包装了原始API，提供了更高级别的抽象。
		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pallet_revive::Code<Hash>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> pallet_revive::ContractInstantiateResult<AccountId, Balance, EventRecord>
		{
			Revive::bare_instantiate(
				RuntimeOrigin::signed(origin),
				value,
				gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block),
				storage_deposit_limit.unwrap_or(u128::MAX),
				code,
				data,
				salt,
				REVIVE_DEBUG_OUTPUT,
				REVIVE_EVENTS,
			)
		}

		/// 上传代码到合约存储中
		///
		/// 此函数允许一个账户上传一段代码到链上，并返回上传结果
		/// 它主要用于部署新的合约代码
		///
		/// # 参数
		///
		/// * `origin` - 上传代码的账户ID
		/// * `code` - 要上传的代码字节序列
		/// * `storage_deposit_limit` - 存储押金限制，可选参数
		///
		/// # 返回值
		///
		/// 返回一个包含代码哈希和可能的余额的上传结果类型
		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
		) -> pallet_revive::CodeUploadResult<Hash, Balance> {
			Revive::bare_upload_code(
				RuntimeOrigin::signed(origin),
				code,
				storage_deposit_limit.unwrap_or(u128::MAX),
			)
		}

		/// 获取指定地址的存储值
		///
		/// 此函数用于检索与给定键关联的存储值对于给定的账户地址
		/// 它主要用于合约开发和调试过程中查询合约存储
		///
		/// # 参数
		///
		/// * `address` - 要查询的账户地址
		/// * `key` - 存储键的字节序列
		///
		/// # 返回值
		///
		/// 返回一个包含存储值的结果类型，如果键不存在，则返回默认值
		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pallet_revive::GetStorageResult {
			Revive::get_storage(
				address,
				key
			)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		/// 根据提供的配置构建区块链的初始状态。
		///
		/// # Parameters
		/// - `config`: 一个字节向量，包含构建初始状态所需的配置信息。
		///
		/// # Returns
		/// 返回一个结果，表示初始状态构建是否成功。
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		/// 获取预设的配置数据。
		///
		/// # Parameters
		/// - `id`: 一个可选的预设ID引用，用于指定所需的预设配置。
		///
		/// # Returns
		/// 返回一个可能包含预设配置数据的字节向量。
		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		/// 获取所有可用的预设名称。
		///
		/// # Returns
		/// 返回一个包含所有预设ID的向量。
		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			Default::default()
		}
	}
}
