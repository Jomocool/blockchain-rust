#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod assets_config;
mod contracts_config;
mod revive_config;
mod weights;
mod xcm_config;

extern crate alloc;

use alloc::vec::Vec;
use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
use polkadot_runtime_common::xcm_sender::NoPriceForMessageDelivery;
use smallvec::smallvec;
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, IdentifyAccount, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature,
};

use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse, TransformOrigin},
	weights::{
		constants::WEIGHT_REF_TIME_PER_SECOND, ConstantMultiplier, Weight, WeightToFeeCoefficient,
		WeightToFeeCoefficients, WeightToFeePolynomial,
	},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use sp_runtime::{MultiAddress, Perbill, Permill};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use xcm_config::{RelayLocation, XcmOriginToTransactDispatchOrigin};

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

use xcm::latest::prelude::BodyId;

// 定义签名类型，用于表示交易或消息的签名
pub type Signature = MultiSignature;

// 定义账户ID类型，通过签名验证来确定账户身份
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

// 定义余额类型，用于表示账户余额，采用128位无符号整数
pub type Balance = u128;

pub type Nonce = u32;

// 定义哈希类型，采用SP核心库中的H256，用于表示256位哈希值
pub type Hash = sp_core::H256;

// 定义块编号类型，使用32位无符号整数来表示区块链中的块编号
pub type BlockNumber = u32;

// 定义地址类型，使用多地址格式封装账户ID，用于表示链上的地址
pub type Address = MultiAddress<AccountId, ()>;

// 定义块头类型，包含块编号和采用BlakeTwo256算法的哈希值
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

// 定义块类型，包含块头和未经检查的交易
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

// 定义签名块类型，包含块及其签名，用于表示已签名的块
pub type SignedBlock = generic::SignedBlock<Block>;

// 定义块ID类型，用于唯一标识区块链中的块
pub type BlockId = generic::BlockId<Block>;

// 定义一个共识钩子类型，用于处理特定的区块链共识算法
// 该类型是通过指定运行时环境、中继链时隙持续时间、区块处理速度和未包含段容量来固定的
type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,                          // 运行时环境，指定了区块链的具体配置和规则
	RELAY_CHAIN_SLOT_DURATION_MILLIS, // 中继链时隙持续时间（毫秒），决定了区块产生的时间间隔
	BLOCK_PROCESSING_VELOCITY,        // 区块处理速度，表示网络处理区块的能力
	UNINCLUDED_SEGMENT_CAPACITY,      // 未包含段容量，限制了未被网络接受的区块数量
>;

/// 定义了一个元组类型SignedExtra，用于存储各种交易检查的集合
///
/// 这个类型的目的是聚合多个用于验证交易合法性的检查模块，以便在
/// 交易处理流程中确保交易符合各种系统和业务规则
///
/// - CheckNonZeroSender: 确保交易的发送者账户余额不为零
/// - CheckSpecVersion: 检查交易的规格版本与节点运行时版本一致
/// - CheckTxVersion: 验证交易的版本号
/// - CheckGenesis: 确认交易中的创世块哈希与节点的创世块哈希匹配
/// - CheckEra: 验证交易的时期（era）信息
/// - CheckNonce: 检查交易的随机数（nonce），以防止重放攻击
/// - CheckWeight: 确保交易的权重在可接受范围内
/// - ChargeTransactionPayment: 处理交易费用的支付
///
/// 这些检查模块共同工作，以确保每笔交易在被网络接受和处理之前都是有效和安全的
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// 定义未检查的交易类型
///
/// 未检查的交易是指尚未经过验证的交易
/// 这个类型别名定义了交易的格式，包括地址、调用、签名和额外的签名信息
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// 定义执行器类型
///
/// 执行器负责处理块中的交易并更新状态
/// 这个类型别名定义了执行器的配置，包括运行时、块、链上下文、运行时和包含所有系统的模块
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

/// `WeightToFee` 结构体实现了 `WeightToFeePolynomial` 特性，
/// 用于将链上操作的权重转换为费用。
pub struct WeightToFee;

impl WeightToFeePolynomial for WeightToFee {
	/// 指定余额的类型。
	type Balance = Balance;

	/// 返回权重到费用的多项式系数。
	///
	/// 该函数定义了如何将操作的权重映射到交易费用的计算公式。
	/// 其中，p 代表了每个单位权重的费用比例，
	/// 而 q 则是基础权重的费用转换因子。
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// 计算每个单位权重的费用比例。
		let p = MILLIUNIT / 10;
		// 计算基础权重的费用转换因子。
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());

		// 构造并返回一个包含多项式系数的向量。
		smallvec![WeightToFeeCoefficient {
			degree: 1,       // 多项式的度为1，表示线性关系。
			negative: false, // 系数为正，表示费用随权重增加而增加。
			// 计算分数部分，表示p除以q的余数与q的比例。
			coeff_frac: Perbill::from_rational(p % q, q),
			// 计算整数部分，表示p除以q的商。
			coeff_integer: p / q,
		}]
	}
}

pub mod opaque {
	use super::*;
	use sp_runtime::{
		generic,
		traits::{BlakeTwo256, Hash as HashT},
	};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	// 定义一个别名Header，代表特定于区块链的区块头类型
	// 区块头包含区块编号和使用BlakeTwo256算法计算的哈希值
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

	// 定义一个别名Block，代表特定于区块链的区块类型
	// 区块包含区块头和一组未经检查的交易
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;

	// 定义一个别名BlockId，代表区块的标识符类型
	// 区块标识符用于唯一标识一个区块
	pub type BlockId = generic::BlockId<Block>;

	// 定义一个别名Hash，代表哈希值的类型
	// 该类型是根据BlakeTwo256哈希算法计算得出的哈希值类型
	pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
	/// `SessionKeys` 结构体用于存储与当前会话相关的密钥信息。
	///
	/// 此结构体在系统中用于身份验证和加密通信，包含了一个 `Aura` 类型的字段。
	/// `Aura` 是一种用于密钥管理的机制，这里用于存储和管理会话密钥。
	///
	/// 字段:
	/// - `aura`: 一个 `Aura` 类型的实例，用于密钥的生成、存储和使用。
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	// 设置规范名称，用于标识运行时的名称
	spec_name: create_runtime_str!("contracts-parachain"),
	// 设置实现名称，用于标识运行时实现的名称
	impl_name: create_runtime_str!("contracts-parachain"),
	// 设置作者版本，用于标识运行时作者的相关版本信息
	authoring_version: 1,
	// 设置规范版本，用于标识运行时规范的版本
	spec_version: 1,
	// 设置实现版本，用于标识运行时实现的版本
	impl_version: 0,
	// 设置运行时API版本，用于标识运行时所支持的API版本
	apis: RUNTIME_API_VERSIONS,
	// 设置事务版本，用于标识运行时中事务的版本
	transaction_version: 1,
	// 设置状态版本，用于标识运行时中状态的版本
	state_version: 1,
};

mod block_times {
	// 定义每个区块的时间间隔为6000毫秒，即6秒
	pub const MILLISECS_PER_BLOCK: u64 = 6000;

	// 定义时隙持续时间与时区块时间间隔相同，为6000毫秒
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
}
pub use block_times::*;

// 定义一个常量MINUTES，表示一分钟内包含的区块数量
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
// 定义一个常量HOURS，表示一小时内包含的区块数量
pub const HOURS: BlockNumber = MINUTES * 60;
// 定义一个常量DAYS，表示一天内包含的区块数量
pub const DAYS: BlockNumber = HOURS * 24;

// 定义一个常量UNIT，代表货币的基本单位
pub const UNIT: Balance = 1_000_000_000_000;
// 定义一个常量MILLIUNIT，代表货币的千分之一单位
pub const MILLIUNIT: Balance = 1_000_000_000;
// 定义一个常量MICROUNIT，代表货币的百万分之一单位
pub const MICROUNIT: Balance = 1_000_000;

// 定义一个常量EXISTENTIAL_DEPOSIT，表示账户存在的最小存款额
pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

// 定义一个常量AVERAGE_ON_INITIALIZE_RATIO，表示初始化时平均占用的比率
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

// 定义一个常量NORMAL_DISPATCH_RATIO，表示正常调度的比率
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

// 定义一个常量MAXIMUM_BLOCK_WEIGHT，表示最大区块权重
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
	cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

mod async_backing_params {
	// 定义未包含段的容量常量，用于指定相关数据结构或缓冲区的固定大小
	pub(crate) const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;

	// 定义块处理速度常量，表示每个时间段内处理的块数量
	pub(crate) const BLOCK_PROCESSING_VELOCITY: u32 = 1;

	// 定义中继链插槽持续时间常量，用于设定中继链上每个插槽的时间长度
	pub(crate) const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;
}
pub(crate) use async_backing_params::*;

#[cfg(feature = "std")]
/// 返回当前的本地版本信息
///
/// 该函数用于获取当前环境的本地版本信息，包括运行时版本和授权信息
/// 它主要用于内部版本管理和调试目的
///
/// # 返回值
/// * `NativeVersion` - 包含当前运行时版本和默认授权信息的结构体
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	// 定义当前运行时的版本常量
	pub const Version: RuntimeVersion = VERSION;

	// 配置区块长度限制
	// 这里设置的最大区块大小为5MB，正常事务占比为预定义的NORMAL_DISPATCH_RATIO
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);

	// 配置区块权重限制
	// 通过BlockWeights::builder()构建区块权重配置
	// .base_block()设置基础区块执行权重
	// .for_class()分别配置不同类别的事务权重限制
	// .avg_block_initialization()设置平均每块初始化比率
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

	// 设置SS58地址编码前缀为42
	pub const SS58Prefix: u16 = 42;
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Runtime {
	// 定义账户ID的类型别名，用于标识账户
	type AccountId = AccountId;
	// 定义Nonce的类型别名，Nonce通常用于事务重放保护
	type Nonce = Nonce;
	// 定义Hash的类型别名，用于标识区块链中的区块哈希
	type Hash = Hash;
	// 定义Block的类型别名，代表区块链中的区块
	type Block = Block;
	// 定义BlockHashCount的类型别名，表示区块哈希的计数
	type BlockHashCount = BlockHashCount;
	// 定义Version的类型别名，代表运行时版本
	type Version = Version;
	// 定义AccountData的类型别名，关联余额模块的账户数据
	type AccountData = pallet_balances::AccountData<Balance>;
	// 定义DbWeight的类型别名，用于表示数据库操作的权重
	type DbWeight = RocksDbWeight;
	// 定义BlockWeights的类型别名，代表区块的权重
	type BlockWeights = RuntimeBlockWeights;
	// 定义BlockLength的类型别名，代表区块的长度
	type BlockLength = RuntimeBlockLength;
	// 定义SS58Prefix的类型别名，SS58Prefix用于编码账户ID
	type SS58Prefix = SS58Prefix;
	// 定义OnSetCode的类型别名，指定当设置新的runtime代码时需要调用的函数
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	// 定义MaxConsumers的类型别名，表示最大消费者数量
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_timestamp::Config for Runtime {
	// 定义Moment类型为64位无符号整数，用于表示时间戳
	type Moment = u64;

	// 指定Aura为时间戳设置时的处理逻辑，通常用于区块的时间戳验证
	type OnTimestampSet = Aura;

	// 定义最小周期为时隙持续时间的一半，用于确保时间戳的更新频率
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;

	// 权重信息类型定义，目前未使用，留空以备未来扩展
	type WeightInfo = ();
}

// 配置pallet_authorship模块
impl pallet_authorship::Config for Runtime {
    // 指定寻找作者的机制
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    // 指定事件处理机制
    type EventHandler = (CollatorSelection,);
}

// 定义存在性存款的参数
parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

// 配置pallet_balances模块
impl pallet_balances::Config for Runtime {
    // 设置最大锁数量
    type MaxLocks = ConstU32<50>;
    // 设置余额类型
    type Balance = Balance;
    // 设置运行时事件
    type RuntimeEvent = RuntimeEvent;
    // 设置尘埃移除机制
    type DustRemoval = ();
    // 设置存在性存款
    type ExistentialDeposit = ExistentialDeposit;
    // 设置账户存储机制
    type AccountStore = System;
    // 设置权重信息
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    // 设置最大储备数量
    type MaxReserves = ConstU32<50>;
    // 设置储备标识符
    type ReserveIdentifier = [u8; 8];
    // 设置运行时持有原因
    type RuntimeHoldReason = RuntimeHoldReason;
    // 设置运行时冻结原因
    type RuntimeFreezeReason = RuntimeFreezeReason;
    // 设置冻结标识符
    type FreezeIdentifier = ();
    // 设置最大冻结数量
    type MaxFreezes = ConstU32<50>;
}

// 定义交易字节费用的参数
parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

// 配置pallet_transaction_payment模块
impl pallet_transaction_payment::Config for Runtime {
    // 设置运行时事件
    type RuntimeEvent = RuntimeEvent;
    // 设置交易收费机制
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    // 设置权重到费用的转换机制
    type WeightToFee = WeightToFee;
    // 设置长度到费用的转换机制
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    // 设置费用调整更新机制
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    // 设置操作费用乘数
    type OperationalFeeMultiplier = ConstU8<5>;
}

// 配置pallet_sudo模块
impl pallet_sudo::Config for Runtime {
    // 设置运行时事件
    type RuntimeEvent = RuntimeEvent;
    // 设置运行时调用
    type RuntimeCall = RuntimeCall;
    // 设置权重信息
    type WeightInfo = ();
}

// 定义权重参数，用于XCMP和DMP消息处理的权重分配
parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

// 实现cumulus_pallet_parachain_system模块的配置接口
impl cumulus_pallet_parachain_system::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
	type ConsensusHook = ConsensusHook;
}

// 实现parachain_info模块的配置接口
impl parachain_info::Config for Runtime {}

// 定义消息队列服务的权重参数
parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

// 实现pallet_message_queue模块的配置接口
impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
		cumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = ConstU32<{ 64 * 1024 }>;
	type MaxStale = ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = ();
}

// 实现cumulus_pallet_aura_ext模块的配置接口
impl cumulus_pallet_aura_ext::Config for Runtime {}

// 实现cumulus_pallet_xcmp_queue模块的配置接口
impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = ();
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	type MaxPageSize = ConstU32<{ 1 << 16 }>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
}

// 定义会话管理相关的参数
parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

// 实现pallet_session模块的配置接口
impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = ();
}

// 配置Aura共识算法的参数
impl pallet_aura::Config for Runtime {
    // 指定权威节点的类型
    type AuthorityId = AuraId;
    // 禁用的验证器类型，这里使用空元组表示不支持禁用验证器功能
    type DisabledValidators = ();
    // 最大权威节点数量
    type MaxAuthorities = ConstU32<100_000>;
    // 是否允许一个时隙内产生多个区块，这里设置为false
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
    // 时隙持续时间，这里设置为最小周期的两倍
    type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Self>;
}

// 定义参数类型
parameter_types! {
    // PotId用于标识PotStake模块
    pub const PotId: PalletId = PalletId(*b"PotStake");
    // SessionLength定义会话长度为6小时
    pub const SessionLength: BlockNumber = 6 * HOURS;
    // StakingAdminBodyId用于标识Staking管理机构
    pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

// 定义验证器选择的更新源类型
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
    EnsureRoot<AccountId>,
    EnsureXcm<IsVoiceOfBody<RelayLocation, StakingAdminBodyId>>,
>;

// 配置CollatorSelection模块的参数
impl pallet_collator_selection::Config for Runtime {
    // 运行时事件类型
    type RuntimeEvent = RuntimeEvent;
    // 用于质押的货币类型
    type Currency = Balances;
    // 验证器选择的更新源
    type UpdateOrigin = CollatorSelectionUpdateOrigin;
    // PotId用于标识CollatorSelection模块
    type PotId = PotId;
    // 最大候选验证器数量
    type MaxCandidates = Const

#[frame_support::runtime]
mod runtime {

	// 定义运行时的结构体，包含了一系列运行时特性
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
	
	// 系统模块，提供基础的运行时功能
	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	// 随机性模块，提供不安全的随机性生成，主要用于测试和开发
	#[runtime::pallet_index(1)]
	pub type RandomnessCollectiveFlip = pallet_insecure_randomness_collective_flip;
	// 实用模块，提供组合和其他实用功能
	#[runtime::pallet_index(2)]
	pub type Utility = pallet_utility;
	// 时间戳模块，允许设置和读取时间戳
	#[runtime::pallet_index(3)]
	pub type Timestamp = pallet_timestamp;
	// 余额模块，处理账户余额和转账逻辑
	#[runtime::pallet_index(4)]
	pub type Balances = pallet_balances;
	// 作者身份模块，用于确定区块的作者
	#[runtime::pallet_index(5)]
	pub type Authorship = pallet_authorship;
	// 交易支付模块，处理交易费用的支付
	#[runtime::pallet_index(6)]
	pub type TransactionPayment = pallet_transaction_payment;
	// 超级用户模块，提供超级用户权限管理
	#[runtime::pallet_index(7)]
	pub type Sudo = pallet_sudo;
	// 合约模块，支持智能合约的执行
	#[runtime::pallet_index(8)]
	pub type Contracts = pallet_contracts;
	// 复活模块，用于处理账户的复活逻辑
	#[runtime::pallet_index(9)]
	pub type Revive = pallet_revive;
	// 资产模块，支持资产的创建和管理
	#[runtime::pallet_index(10)]
	pub type Assets = pallet_assets;
	// 平行链系统模块，提供平行链系统级别的功能
	#[runtime::pallet_index(11)]
	pub type ParachainSystem = cumulus_pallet_parachain_system;
	// 平行链信息模块，提供平行链的基本信息
	#[runtime::pallet_index(12)]
	pub type ParachainInfo = parachain_info;
	
	// 收集者选择模块，用于选择平行链的收集者
	#[runtime::pallet_index(13)]
	pub type CollatorSelection = pallet_collator_selection;
	// 会话模块，管理验证器的会话密钥
	#[runtime::pallet_index(14)]
	pub type Session = pallet_session;
	// Aura模块，提供Aura共识的实现
	#[runtime::pallet_index(15)]
	pub type Aura = pallet_aura;
	// Aura扩展模块，对Aura共识的扩展
	#[runtime::pallet_index(16)]
	pub type AuraExt = cumulus_pallet_aura_ext;
	// XCMP队列模块，处理跨链消息传递的队列
	#[runtime::pallet_index(17)]
	pub type XcmpQueue = cumulus_pallet_xcmp_queue;
	// Polkadot XCM模块，支持Polkadot的跨链通信
	#[runtime::pallet_index(18)]
	pub type PolkadotXcm = pallet_xcm;
	// Cumulus XCM模块，支持Cumulus的跨链通信
	#[runtime::pallet_index(19)]
	pub type CumulusXcm = cumulus_pallet_xcm;
	// 消息队列模块，处理消息队列
	#[runtime::pallet_index(20)]
	pub type MessageQueue = pallet_message_queue;
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_message_queue, MessageQueue]
		[pallet_sudo, Sudo]
		[pallet_collator_selection, CollatorSelection]
		[cumulus_pallet_parachain_system, ParachainSystem]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
	);
}

// 定义一个别名EventRecord，用于表示系统框架中的事件记录
// 它包含了事件的类型和哈希值
type EventRecord = frame_system::EventRecord<
	<Runtime as frame_system::Config>::RuntimeEvent,
	<Runtime as frame_system::Config>::Hash,
>;

// 设置合约模块的调试输出级别为UnsafeDebug，这可能暴露敏感信息
const CONTRACTS_DEBUG_OUTPUT: pallet_contracts::DebugInfo =
	pallet_contracts::DebugInfo::UnsafeDebug;

// 设置合约模块的事件收集级别为UnsafeCollect，这可能收集到未过滤的事件信息
const CONTRACTS_EVENTS: pallet_contracts::CollectEvents =
	pallet_contracts::CollectEvents::UnsafeCollect;

// 设置Revive模块的调试输出级别为UnsafeDebug，同样可能暴露敏感信息
const REVIVE_DEBUG_OUTPUT: pallet_revive::DebugInfo = pallet_revive::DebugInfo::UnsafeDebug;

// 设置Revive模块的事件收集级别为UnsafeCollect，这可能收集到未过滤的事件信息
const REVIVE_EVENTS: pallet_revive::CollectEvents = pallet_revive::CollectEvents::UnsafeCollect;

impl_runtime_apis! {
		// 实现Aura共识算法的API，用于块和AuraId类型
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
	    // 返回 Aura 的时隙持续时间
	    fn slot_duration() -> sp_consensus_aura::SlotDuration {
	        sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
	    }
	
	    // 返回当前的Aura权威列表
	    fn authorities() -> Vec<AuraId> {
	        pallet_aura::Authorities::<Runtime>::get().into_inner()
	    }
	}
	
	// 实现用于处理Aura未包含段的API
	impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
	    // 判断在给定的时隙上是否可以构建块
	    fn can_build_upon(
	        included_hash: <Block as BlockT>::Hash,
	        slot: cumulus_primitives_aura::Slot
	    ) -> bool {
	        ConsensusHook::can_build_upon(included_hash, slot)
	    }
	}
	
	// 实现核心API，提供运行时版本信息和块执行功能
	impl sp_api::Core<Block> for Runtime {
	    // 返回运行时的版本信息
	    fn version() -> RuntimeVersion {
	        VERSION
	    }
	
	    // 执行一个完整的块
	    fn execute_block(block: Block) {
	        Executive::execute_block(block)
	    }
	
	    // 初始化一个块，返回外在数据包含模式
	    fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
	        Executive::initialize_block(header)
	    }
	}

	// 实现sp_api::Metadata trait，提供运行时元数据相关功能
	impl sp_api::Metadata<Block> for Runtime {
	    // 返回最新版本的元数据
	    fn metadata() -> OpaqueMetadata {
	        OpaqueMetadata::new(Runtime::metadata().into())
	    }
	
	    // 根据版本号返回特定版本的元数据
	    fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
	        Runtime::metadata_at_version(version)
	    }
	
	    // 返回所有可用的元数据版本号
	    fn metadata_versions() -> Vec<u32> {
	        Runtime::metadata_versions()
	    }
	}
	
	// 实现sp_block_builder::BlockBuilder trait，提供构建区块的功能
	impl sp_block_builder::BlockBuilder<Block> for Runtime {
	    // 应用一个外部交易到当前区块中
	    fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
	        Executive::apply_extrinsic(extrinsic)
	    }
	
	    // 完成当前区块的构建并返回区块头
	    fn finalize_block() -> <Block as BlockT>::Header {
	        Executive::finalize_block()
	    }
	
	    // 根据内在数据生成内在外部交易
	    fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
	        data.create_extrinsics()
	    }
	
	    // 检查区块中的内在外部交易是否有效
	    fn check_inherents(
	        block: Block,
	        data: sp_inherents::InherentData,
	    ) -> sp_inherents::CheckInherentsResult {
	        data.check_extrinsics(&block)
	    }
	}
	
	// 实现sp_transaction_pool::runtime_api::TaggedTransactionQueue trait，提供交易有效性检查
	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
	    // 验证交易的有效性
	    fn validate_transaction(
	        source: TransactionSource,
	        tx: <Block as BlockT>::Extrinsic,
	        block_hash: <Block as BlockT>::Hash,
	    ) -> TransactionValidity {
	        Executive::validate_transaction(source, tx, block_hash)
	    }
	}
	
	// 实现sp_offchain::OffchainWorkerApi trait，提供离线工作器功能
	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
	    // 执行离线工作器任务
	    fn offchain_worker(header: &<Block as BlockT>::Header) {
	        Executive::offchain_worker(header)
	    }
	}
	
	// 实现sp_session::SessionKeys trait，管理会话密钥
	impl sp_session::SessionKeys<Block> for Runtime {
	    // 生成会话密钥
	    fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
	        SessionKeys::generate(seed)
	    }
	
	    // 解码会话密钥
	    fn decode_session_keys(
	        encoded: Vec<u8>,
	    ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
	        SessionKeys::decode_into_raw_public_keys(&encoded)
	    }
	}

	// 实现AccountNonceApi trait，提供获取账户nonce的功能
	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
	    // 获取指定账户的nonce
	    fn account_nonce(account: AccountId) -> Nonce {
	        System::account_nonce(account)
	    }
	}
	
	// 实现TransactionPaymentApi trait，提供查询交易费用相关的信息
	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
	    // 查询交易的派发信息
	    fn query_info(
	        uxt: <Block as BlockT>::Extrinsic,
	        len: u32,
	    ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
	        TransactionPayment::query_info(uxt, len)
	    }
	    // 查询交易的费用详情
	    fn query_fee_details(
	        uxt: <Block as BlockT>::Extrinsic,
	        len: u32,
	    ) -> pallet_transaction_payment::FeeDetails<Balance> {
	        TransactionPayment::query_fee_details(uxt, len)
	    }
	    // 根据权重查询费用
	    fn query_weight_to_fee(weight: Weight) -> Balance {
	        TransactionPayment::weight_to_fee(weight)
	    }
	    // 根据长度查询费用
	    fn query_length_to_fee(length: u32) -> Balance {
	        TransactionPayment::length_to_fee(length)
	    }
	}
	
	// 实现TransactionPaymentCallApi trait，提供基于RuntimeCall的查询交易费用相关的信息
	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
	    for Runtime
	{
	    // 查询调用的派发信息
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
	    // 根据权重查询费用
	    fn query_weight_to_fee(weight: Weight) -> Balance {
	        TransactionPayment::weight_to_fee(weight)
	    }
	    // 根据长度查询费用
	    fn query_length_to_fee(length: u32) -> Balance {
	        TransactionPayment::length_to_fee(length)
	    }
	}
	
	// 实现CollectCollationInfo trait，提供收集collation信息的功能
	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
	    // 收集collation信息
	    fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
	        ParachainSystem::collect_collation_info(header)
	    }
	}
	
	// 当启用"try-runtime"功能时，实现TryRuntime trait，提供测试运行时升级和执行区块的功能
	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
	    // 执行运行时升级，并返回升级所花费的权重
	    fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
	        let weight = Executive::try_runtime_upgrade(checks).unwrap();
	        (weight, RuntimeBlockWeights::get().max_block)
	    }
	
	    // 尝试执行区块，并返回执行所花费的权重
	    fn execute_block(
	        block: Block,
	        state_root_check: bool,
	        signature_check: bool,
	        select: frame_try_runtime::TryStateSelect,
	    ) -> Weight {
	        Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
	    }
	}

		// 当运行时包含"runtime-benchmarks"特性时，实现Benchmark<Block> trait for Runtime
	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
	    // 返回基准测试的元数据和存储信息
	    fn benchmark_metadata(extra: bool) -> (
	        Vec<frame_benchmarking::BenchmarkList>,
	        Vec<frame_support::traits::StorageInfo>,
	    ) {
	        // 使用必要的trait和模块
	        use frame_benchmarking::{Benchmarking, BenchmarkList};
	        use frame_support::traits::StorageInfoTrait;
	        use frame_system_benchmarking::Pallet as SystemBench;
	        use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
	
	        // 初始化一个空的BenchmarkList向量
	        let mut list = Vec::<BenchmarkList>::new();
	        // 列出所有基准测试，并根据extra参数决定是否包括额外的基准测试
	        list_benchmarks!(list, extra);
	
	        // 获取所有模块的存储信息
	        let storage_info = AllPalletsWithSystem::storage_info();
	        // 返回基准测试列表和存储信息
	        (list, storage_info)
	    }
	
	    // 执行基准测试调度
	    fn dispatch_benchmark(
	        config: frame_benchmarking::BenchmarkConfig
	    ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
	        // 使用必要的trait和模块
	        use frame_benchmarking::{BenchmarkError, Benchmarking, BenchmarkBatch};
	
	        // 为了系统基准测试的配置
	        use frame_system_benchmarking::Pallet as SystemBench;
	        // 实现系统基准测试配置trait for Runtime
	        impl frame_system_benchmarking::Config for Runtime {
	            // 设置代码基准测试的要求
	            fn setup_set_code_requirements(code: &Vec<u8>) -> Result<(), BenchmarkError> {
	                // 初始化系统以进行设置代码基准测试
	                ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
	                Ok(())
	            }
	
	            // 验证设置代码的基准测试
	            fn verify_set_code() {
	                // 确认最后的事件是验证函数已存储
	                System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
	            }
	        }
	
	        // 为了会话基准测试的配置
	        use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
	        // 实现会话基准测试配置trait for Runtime
	        impl cumulus_pallet_session_benchmarking::Config for Runtime {}
	
	        // 使用白名单存储键
	        use frame_support::traits::WhitelistedStorageKeys;
	        let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();
	
	        // 初始化一个空的BenchmarkBatch向量
	        let mut batches = Vec::<BenchmarkBatch>::new();
	        // 创建参数元组，包含配置和白名单
	        let params = (&config, &whitelist);
	        // 添加所有基准测试到批次中
	        add_benchmarks!(params, batches);
	
	        // 如果批次为空，则返回错误
	        if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
	        // 否则，返回批次
	        Ok(batches)
	    }
	}

// 实现ReviveApi接口，提供合约调用、实例化和代码上传等功能
impl pallet_revive::ReviveApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord> for Runtime
{
    // 调用已部署的合约
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

    // 实例化一个新的合约
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

    // 上传合约代码到链上
    fn upload_code(
        origin: AccountId,
        code: Vec<u8>,
        storage_deposit_limit: Option<Balance>,
    ) -> pallet_revive::CodeUploadResult<Hash, Balance>
    {
        Revive::bare_upload_code(
            RuntimeOrigin::signed(origin),
            code,
            storage_deposit_limit.unwrap_or(u128::MAX),
        )
    }

    // 获取合约存储中的值
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

// 实现ContractsApi接口，提供类似的合约操作功能
impl pallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord>
    for Runtime
{
    // 调用已部署的合约
    fn call(
        origin: AccountId,
        dest: AccountId,
        value: Balance,
        gas_limit: Option<Weight>,
        storage_deposit_limit: Option<Balance>,
        input_data: Vec<u8>,
    ) -> pallet_contracts::ContractExecResult<Balance, EventRecord> {
        let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
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

    // 实例化一个新的合约
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
        let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
        Contracts::bare_instantiate(
            origin,
            value,
            gas_limit,
            storage_deposit_limit,
            code,
            data,
            salt,
            CONTRACTS_DEBUG_OUTPUT,
            CONTRACTS_EVENTS,
        )
    }

    // 上传合约代码到链上
    fn upload_code(
        origin: AccountId,
        code: Vec<u8>,
        storage_deposit_limit: Option<Balance>,
        determinism: pallet_contracts::Determinism,
    ) -> pallet_contracts::CodeUploadResult<Hash, Balance>
    {
        Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
    }

    // 获取合约存储中的值
    fn get_storage(
        address: AccountId,
        key: Vec<u8>,
    ) -> pallet_contracts::GetStorageResult {
        Contracts::get_storage(address, key)
    }
}

// 实现GenesisBuilder接口，用于构建创世区块状态
impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
    fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
        build_state::<RuntimeGenesisConfig>(config)
    }

    fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
        get_preset::<RuntimeGenesisConfig>(id, |_| None)
    }

    fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
        Default::default()
    }
}

// 注册验证块的逻辑，用于 Aura 扩展
cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}