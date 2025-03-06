use super::{
	AccountId, AllPalletsWithSystem, Balances, ParachainInfo, ParachainSystem, PolkadotXcm,
	Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, WeightToFee, XcmpQueue,
};
use frame_support::{
	parameter_types,
	traits::{ConstU32, Contains, Everything, Nothing},
	weights::Weight,
};
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use polkadot_runtime_common::impls::ToAuthor;
use xcm::latest::prelude::*;
#[allow(deprecated)]
use xcm_builder::CurrencyAdapter;
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowTopLevelPaidExecutionFrom,
	DenyReserveTransferToRelayChain, DenyThenTry, EnsureXcmOrigin, FixedWeightBounds,
	FrameTransactionalProcessor, IsConcrete, NativeAsset, ParentIsPreset, RelayChainAsNative,
	SiblingParachainAsNative, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, TakeWeightCredit, TrailingSetTopicAsId, UsingComponents,
	WithComputedOrigin, WithUniqueTopic,
};
use xcm_executor::XcmExecutor;

parameter_types! {
	// 定义Token的位置，用于标识令牌在跨链消息中的位置
	pub const TokenLocation: Location = Here.into_location();

	// 定义中继链的位置，用于标识中继链在跨链消息中的位置
	pub const RelayLocation: Location = Location::parent();

	// 定义中继链的网络ID，当前设置为None，表示未指定网络
	pub const RelayNetwork: Option<NetworkId> = None;

	// 定义中继链的起源，用于标识中继链在跨链消息中的起源
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();

	// 定义通用位置，用于标识在跨链消息中的通用位置
	// 这里特别指出了当前平行链的ID
	pub UniversalLocation: InteriorLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

// 定义一个类型别名LocationToAccountId，用于将位置信息映射到账户ID
// 该别名结合了AccountId32Aliases和ParentIsPreset的特性
pub type LocationToAccountId =
	(AccountId32Aliases<RelayNetwork, AccountId>, ParentIsPreset<AccountId>);

// 定义一个本地资产交易处理器类型LocalAssetTransactor
// 使用CurrencyAdapter适配Balances模块，处理与RelayLocation相关的资产
#[allow(deprecated)]
pub type LocalAssetTransactor =
	CurrencyAdapter<Balances, IsConcrete<RelayLocation>, LocationToAccountId, AccountId, ()>;

// 定义一个本地余额交易处理器类型LocalBalancesTransactor
// 使用CurrencyAdapter适配Balances模块，处理与TokenLocation相关的资产
#[allow(deprecated)]
pub type LocalBalancesTransactor =
	CurrencyAdapter<Balances, IsConcrete<TokenLocation>, LocationToAccountId, AccountId, ()>;

// 定义一个资产交易处理器的集合类型AssetTransactors
// 包含LocalBalancesTransactor和LocalAssetTransactor，用于处理不同类型的资产交易
pub type AssetTransactors = (LocalBalancesTransactor, LocalAssetTransactor);

// 定义一个XCM起源到交易调度起源的转换器类型XcmOriginToTransactDispatchOrigin
// 该类型是一个元组，包含了多种起源转换方式，用于处理不同来源的XCM消息
pub type XcmOriginToTransactDispatchOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	// 定义单位重量成本，用于计算执行权重或费用
	// 这里的权重包括了执行时间和存储空间的消耗
	pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);

	// 定义最大指令数量，限制了在一定时间内可以执行的最大指令数
	// 这是为了防止过载和确保系统稳定性
	pub const MaxInstructions: u32 = 100;

	// 定义可以转入持有的最大资产数量，限制了单次操作中可以涉及的资产数量
	// 这有助于管理资源使用并防止滥用
	pub const MaxAssetsIntoHolding: u32 = 64;
}

/// `ParentOrParentsPlurality` 公用结构体用于表示父级或父级复数情况
///
/// 该结构体实现了 `Contains<Location>` trait，用于检查位置是否满足特定条件
pub struct ParentOrParentsPlurality;

impl Contains<Location> for ParentOrParentsPlurality {
	/// 检查给定位置是否为单个父级或单个父级复数情况
	///
	/// ## 参数
	/// - `location`: 指向要检查的位置的引用
	///
	/// ## 返回值
	/// - 如果位置为单个父级或单个父级复数情况，则返回 `true`
	/// - 否则返回 `false`
	fn contains(location: &Location) -> bool {
		// 使用模式匹配来判断位置是否为单个父级或单个父级复数情况
		// (1, []) 表示单个父级
		// (1, [Plurality { .. }]) 表示单个父级复数情况
		matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
	}
}

/// 定义一个类型别名Barrier，用于设置跨链消息的过滤条件
/// 这个类型别名代表了一组复杂的逻辑，用于决定是否允许某个跨链消息通过
/// 它首先拒绝某些类型的转移操作，然后尝试其他条件，最终决定是否允许消息通过
pub type Barrier = TrailingSetTopicAsId<
	DenyThenTry<
		DenyReserveTransferToRelayChain,
		(
			TakeWeightCredit,
			WithComputedOrigin<
				(
					AllowTopLevelPaidExecutionFrom<Everything>,
					AllowExplicitUnpaidExecutionFrom<ParentOrParentsPlurality>,
				),
				UniversalLocation,
				ConstU32<8>,
			>,
		),
	>,
>;

// 定义XcmConfig结构体，用于配置XCM执行器的参数
pub struct XcmConfig;

// 实现xcm_executor::Config trait，以配置XCM执行器
impl xcm_executor::Config for XcmConfig {
	// 指定运行时调用的类型
	type RuntimeCall = RuntimeCall;
	// 指定XCM消息的发送者类型
	type XcmSender = XcmRouter;
	// 指定资产交易者类型，负责处理资产的转移
	type AssetTransactor = AssetTransactors;
	// 指定起源转换器类型，用于将XCM起源转换为可执行的分发起源
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	// 指定储备资产的类型
	type IsReserve = NativeAsset;
	// 指定远程传输资产的类型，这里留空
	type IsTeleporter = ();
	// 指定通用位置的类型，用于定义XCM消息的来源和目的地
	type UniversalLocation = UniversalLocation;
	// 指定障碍的类型，用于过滤进入的XCM消息
	type Barrier = Barrier;
	// 指定权重计算器的类型，用于计算XCM指令的权重
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	// 指定交易者的类型，用于处理XCM消息中的交易
	type Trader =
		UsingComponents<WeightToFee, RelayLocation, AccountId, Balances, ToAuthor<Runtime>>;
	// 指定响应处理器的类型，用于处理来自其他共识系统的响应消息
	type ResponseHandler = PolkadotXcm;
	// 指定资产陷阱的类型，用于捕获无法处理的资产
	type AssetTrap = PolkadotXcm;
	// 指定资产声明的类型，用于声明对无法处理的资产的所有权
	type AssetClaims = PolkadotXcm;
	// 指定订阅服务的类型，用于管理XCM消息的订阅
	type SubscriptionService = PolkadotXcm;
	// 指定 pallet 实例信息的类型，用于提供系统中所有pallet的信息
	type PalletInstancesInfo = AllPalletsWithSystem;
	// 指定最大资产持有量的类型，用于限制转入持有的资产数量
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	// 指定资产锁定器的类型，这里留空
	type AssetLocker = ();
	// 指定资产交换器的类型，这里留空
	type AssetExchanger = ();
	// 指定费用管理器的类型，这里留空
	type FeeManager = ();
	// 指定消息导出器的类型，这里留空
	type MessageExporter = ();
	// 指定通用别名的类型，这里设置为None
	type UniversalAliases = Nothing;
	// 指定调用分发器的类型，用于分发XCM消息中的调用
	type CallDispatcher = RuntimeCall;
	// 指定安全调用过滤器的类型，用于过滤XCM消息中的调用
	type SafeCallFilter = Everything;
	// 指定别名器的类型，这里留空
	type Aliasers = Nothing;
	// 指定事务处理器的类型，用于处理XCM消息中的事务
	type TransactionalProcessor = FrameTransactionalProcessor;
	// 指定HRMP新通道开放请求处理器的类型，这里留空
	type HrmpNewChannelOpenRequestHandler = ();
	// 指定HRMP通道接受处理器的类型，这里留空
	type HrmpChannelAcceptedHandler = ();
	// 指定HRMP通道关闭处理器的类型，这里留空
	type HrmpChannelClosingHandler = ();
	// 指定XCM记录器的类型，用于记录XCM消息的信息
	type XcmRecorder = PolkadotXcm;
}

/// 定义一个类型别名LocalOriginToLocation，用于将签名的运行时源转换为32字节的账户ID。
/// 这在跨链消息传递中用于标识消息的发送者。
/// - RuntimeOrigin: 运行时源，表示消息或调用的起源。
/// - AccountId: 账户ID类型，通常用于标识用户或账户。
/// - RelayNetwork: 中继链网络标识符，用于区分不同的中继链。
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// 定义XcmRouter类型，用于跨链消息的路由。
/// 该路由器使用唯一主题的组合，允许消息通过中继链的UMP机制发送，
/// 同时也支持通过XCMP队列发送消息。
/// - cumulus_primitives_utility::ParentAsUmp: 使用中继链作为UMP消息的父级。
/// - ParachainSystem: 用于访问和管理平行链系统相关信息。
/// - (): 表示无操作或默认操作。
/// - XcmpQueue: XCMP消息队列，用于处理平行链间的异步消息传递。
pub type XcmRouter =
	WithUniqueTopic<(cumulus_primitives_utility::ParentAsUmp<ParachainSystem, (), ()>, XcmpQueue)>;

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<Location> = Some(Parent.into());
}

// 实现pallet_xcm配置接口，用于配置Runtime的跨共识消息（XCM）功能
impl pallet_xcm::Config for Runtime {
    // 指定RuntimeEvent类型，用于编码和解码Runtime事件
	type RuntimeEvent = RuntimeEvent;
    // 定义发送XCM消息的起源检查方式
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    // 指定XCM路由的实现，负责跨共识消息的传递
	type XcmRouter = XcmRouter;
    // 定义执行XCM消息的起源检查方式
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    // 允许执行所有类型的XCM消息，无过滤
	type XcmExecuteFilter = Everything;
    // 指定XCM执行器的配置，包括消息执行的权重和费用计算方式
	type XcmExecutor = XcmExecutor<XcmConfig>;
    // 允许所有类型的XCM消息进行远程传输，无过滤
	type XcmTeleportFilter = Everything;
    // 不允许任何类型的XCM消息进行保留转账
	type XcmReserveTransferFilter = Nothing;
    // 定义XCM消息执行权重的计算方式
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    // 定义Runtime的全局位置，用于在不同共识系统中标识Runtime
	type UniversalLocation = UniversalLocation;
    // 指定RuntimeOrigin类型，用于编码和解码Runtime的起源
	type RuntimeOrigin = RuntimeOrigin;
    // 指定RuntimeCall类型，用于编码和解码Runtime的调用
	type RuntimeCall = RuntimeCall;

    // 定义版本发现队列的大小，用于管理XCM版本发现过程中的消息
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    // 指定当前广告的XCM版本
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    // 定义用于支付XCM消息费用的货币类型
	type Currency = Balances;
    // 未使用CurrencyMatcher，留空
	type CurrencyMatcher = ();
    // 未使用TrustedLockers，留空
	type TrustedLockers = ();
    // 定义Location到AccountId的转换方式
	type SovereignAccountOf = LocationToAccountId;
    // 限制最大锁持有者数量
	type MaxLockers = ConstU32<8>;
    // 用于测试的XCM执行权重信息
	type WeightInfo = pallet_xcm::TestWeightInfo;
    // 定义管理员起源，使用Root权限的AccountId
	type AdminOrigin = EnsureRoot<AccountId>;
    // 限制最大远程锁消费者数量为0，不允许远程锁消费
	type MaxRemoteLockConsumers = ConstU32<0>;
    // 未使用RemoteLockConsumerIdentifier，留空
	type RemoteLockConsumerIdentifier = ();
}

// 实现cumulus_pallet_xcm配置接口，进一步扩展Runtime的XCM功能
impl cumulus_pallet_xcm::Config for Runtime {
    // 指定RuntimeEvent类型，用于编码和解码Runtime事件
	type RuntimeEvent = RuntimeEvent;
    // 指定XCM执行器的配置，包括消息执行的权重和费用计算方式
	type XcmExecutor = XcmExecutor<XcmConfig>;
}
