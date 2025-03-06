// 导入项目内定义的模块
use crate::{
    Balance, Balances, BalancesCall, Perbill, Runtime, RuntimeCall, RuntimeEvent,
    RuntimeHoldReason, Timestamp,
};
// 导入frame_support库的特定模块
use frame_support::{
    parameter_types,
    traits::{ConstBool, ConstU32},
};
// 导入frame_system库的特定模块
use frame_system::EnsureSigned;

// 定义一个枚举，用于标识允许的余额调用
pub enum AllowBalancesCall {}

// 实现Contains trait，以定义哪些RuntimeCall被AllowBalancesCall包含
impl frame_support::traits::Contains<RuntimeCall> for AllowBalancesCall {
    fn contains(call: &RuntimeCall) -> bool {
        matches!(call, RuntimeCall::Balances(BalancesCall::transfer_allow_death { .. }))
    }
}

// 定义单位和次单位的余额值
const UNIT: Balance = 1_000_000_000_000;
const MILLIUNIT: Balance = 1_000_000_000;

// 定义一个常量函数，用于计算存款金额
const fn deposit(items: u32, bytes: u32) -> Balance {
    (items as Balance * UNIT + (bytes as Balance) * (5 * MILLIUNIT / 100)) / 10
}

// 使用parameter_types!宏来定义一些参数类型
// 定义与pallet_revive相关的参数类型
parameter_types! {
    // 每个存储项的押金
    pub const DepositPerItem: Balance = deposit(1, 0);
    // 每个字节的押金
    pub const DepositPerByte: Balance = deposit(0, 1);
    // 默认的押金限制
    pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
    // 代码哈希锁定押金的百分比
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
    // 最大代理依赖数量
    pub const MaxDelegateDependencies: u32 = 32;
}

// 实现pallet_revive的Config trait，为Runtime定义相关配置
impl pallet_revive::Config for Runtime {
    // 时间戳模块
    type Time = Timestamp;
    // 货币模块
    type Currency = Balances;
    // 运行时事件
    type RuntimeEvent = RuntimeEvent;
    // 运行时调用
    type RuntimeCall = RuntimeCall;
    // 调用过滤器，允许余额调用
    type CallFilter = AllowBalancesCall;
    // 每个存储项的押金类型
    type DepositPerItem = DepositPerItem;
    // 每个字节的押金类型
    type DepositPerByte = DepositPerByte;
    // 权重价格，用于交易费用计算
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    // 权重信息，用于交易费用计算
    type WeightInfo = pallet_revive::weights::SubstrateWeight<Self>;
    // 链扩展，目前为空
    type ChainExtension = ();
    // 地址生成器，使用默认实现
    type AddressGenerator = pallet_revive::DefaultAddressGenerator;
    // 最大代码长度限制
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    // 运行时内存限制
    type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
    // PVF（便携式验证者框架）内存限制
    type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
    // 是否启用不安全不稳定接口的标志
    type UnsafeUnstableInterface = ConstBool<true>;
    // 代码哈希锁定押金的百分比类型
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    // 运行时持有原因
    type RuntimeHoldReason = RuntimeHoldReason;
    // 调试接口，目前为空
    type Debug = ();
    // 迁移模块，目前为空
    type Migrations = ();
    // 根据是否启用parachain特性选择Xcm模块实现
    #[cfg(feature = "parachain")]
    type Xcm = pallet_xcm::Pallet<Self>;
    #[cfg(not(feature = "parachain"))]
    type Xcm = ();
    // 上传源，确保是签名账户
    type UploadOrigin = EnsureSigned<Self::AccountId>;
    // 实例化源，确保是签名账户
    type InstantiateOrigin = EnsureSigned<Self::AccountId>;
}