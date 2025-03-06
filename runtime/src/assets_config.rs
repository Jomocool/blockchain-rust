use crate::{AccountId, Balance, Balances, Runtime, RuntimeEvent};
use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU32},
};
use frame_system::EnsureSigned;

// 定义货币单位常量，用于表示不同面额的金额
// 每个单位之间的换算关系为：1 DOLLAR = 100 CENTS = 100_000 MILLICENTS
pub const MILLICENTS: Balance = 1_000_000_000; // 1/1000 CENTS
pub const CENTS: Balance = 1_000 * MILLICENTS; // 1/100 DOLLARS
pub const DOLLARS: Balance = 100 * CENTS; // 主要货币单位

// 定义资产相关的常量，用于配置资产模块的各项参数
parameter_types! {
	/// 创建资产时需要支付的押金
	pub const AssetDeposit: Balance = 100 * DOLLARS;

	/// 批准资产转移时需要支付的押金
	pub const ApprovalDeposit: Balance = 1 * DOLLARS;

	/// 资产名称和符号的最大长度限制
	pub const StringLimit: u32 = 50;

	/// 创建资产元数据时的基础押金
	pub const MetadataDepositBase: Balance = 10 * DOLLARS;

	/// 每个字节的元数据需要支付的额外押金
	pub const MetadataDepositPerByte: Balance = 1 * DOLLARS;
}

/// 实现Runtime的pallet_assets::Config配置，定义了资产模块的行为和参数
impl pallet_assets::Config for Runtime {
	/// 运行时事件类型，用于触发和监听资产模块的事件
	type RuntimeEvent = RuntimeEvent;

	/// 资产余额类型，使用u128表示支持大数值
	type Balance = u128;

	/// 资产ID类型，使用u32表示资产标识符
	type AssetId = u32;

	/// 资产ID参数类型，使用Compact编码以节省存储空间
	type AssetIdParameter = codec::Compact<u32>;

	/// 创建资产的来源验证，确保只有签名账户可以创建资产
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;

	/// 货币模块，用于处理资产的余额管理
	type Currency = Balances;

	/// 强制操作的来源验证，仅允许根账户执行强制操作
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;

	/// 创建资产时需要支付的押金
	type AssetDeposit = AssetDeposit;

	/// 创建资产账户时需要支付的押金
	type AssetAccountDeposit = ConstU128<DOLLARS>;

	/// 创建资产元数据时的基础押金
	type MetadataDepositBase = MetadataDepositBase;

	/// 每个字节的元数据需要支付的额外押金
	type MetadataDepositPerByte = MetadataDepositPerByte;

	/// 批准资产转移时需要支付的押金
	type ApprovalDeposit = ApprovalDeposit;

	/// 资产名称和符号的最大长度限制
	type StringLimit = StringLimit;

	/// 冻结功能的实现，默认为空，表示不启用冻结功能
	type Freezer = ();

	/// 额外的配置项，默认为空
	type Extra = ();

	/// 权重信息，用于计算和优化模块的操作成本
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;

	/// 移除项目时的最大数量限制
	type RemoveItemsLimit = ConstU32<1000>;

	/// 回调处理函数，默认为空
	type CallbackHandle = ();

	/// 性能基准测试辅助功能，仅在启用runtime-benchmarks特性时有效
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}
