use crate::{
    Balance, Balances, BalancesCall, Perbill, RandomnessCollectiveFlip, Runtime, RuntimeCall,
    RuntimeEvent, RuntimeHoldReason, Timestamp,
};
use frame_support::{
    parameter_types,
    traits::{ConstBool, ConstU32},
};
use frame_system::EnsureSigned;

// 定义一个枚举，用于标识允许的余额调用
pub enum AllowBalancesCall {}

// 实现Contains trait，以定义哪些RuntimeCall被AllowBalancesCall包含
impl frame_support::traits::Contains<RuntimeCall> for AllowBalancesCall {
    fn contains(call: &RuntimeCall) -> bool {
        matches!(call, RuntimeCall::Balances(BalancesCall::transfer_allow_death { .. }))
    }
}

// 定义单位和次单位的余额
const UNIT: Balance = 1_000_000_000_000;
const MILLIUNIT: Balance = 1_000_000_000;

// 计算存款金额的函数
const fn deposit(items: u32, bytes: u32) -> Balance {
    (items as Balance * UNIT + (bytes as Balance) * (5 * MILLIUNIT / 100)) / 10
}

// 生成pallet_contracts的Schedule配置
fn schedule<T: pallet_contracts::Config>() -> pallet_contracts::Schedule<T> {
    pallet_contracts::Schedule {
        limits: pallet_contracts::Limits {
            runtime_memory: 1024 * 1024 * 1024,
            validator_runtime_memory: 1024 * 1024 * 1024 * 2,
            ..Default::default()
        },
        ..Default::default()
    }
}

// 使用parameter_types!宏来定义参数类型
parameter_types! {
    pub const DepositPerItem: Balance = deposit(1, 0);
    pub const DepositPerByte: Balance = deposit(0, 1);
    pub Schedule: pallet_contracts::Schedule<Runtime> = schedule::<Runtime>();
    pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
    pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
    pub const MaxDelegateDependencies: u32 = 32;
}

// 实现pallet_contracts的Config trait
impl pallet_contracts::Config for Runtime {
        // 定义时间类型为Timestamp
    type Time = Timestamp;
    // 定义随机性类型为RandomnessCollectiveFlip
    type Randomness = RandomnessCollectiveFlip;
    // 定义货币类型为Balances
    type Currency = Balances;
    // 定义运行时事件类型为RuntimeEvent
    type RuntimeEvent = RuntimeEvent;
    // 定义运行时调用类型为RuntimeCall
    type RuntimeCall = RuntimeCall;
    
    // 定义调用过滤器类型为AllowBalancesCall
    type CallFilter = AllowBalancesCall;
    // 定义每项存款类型为DepositPerItem
    type DepositPerItem = DepositPerItem;
    // 定义每字节存款类型为DepositPerByte
    type DepositPerByte = DepositPerByte;
    // 定义调用堆栈类型为一个包含23个Frame的数组
    type CallStack = [pallet_contracts::Frame<Self>; 23];
    // 定义权重价格类型为Pallet<Self>
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    // 定义权重信息类型为SubstrateWeight<Self>
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    // 定义链扩展类型为()
    type ChainExtension = ();
    // 定义调度类型为Schedule
    type Schedule = Schedule;
    // 定义地址生成器类型为DefaultAddressGenerator
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    // 定义最大代码长度类型为ConstU32<{ 256 * 1024 }>
    type MaxCodeLen = ConstU32<{ 256 * 1024 }>;
    // 定义默认存款限制类型为DefaultDepositLimit
    type DefaultDepositLimit = DefaultDepositLimit;
    // 定义最大存储键长度类型为ConstU32<128>
    type MaxStorageKeyLen = ConstU32<128>;
    // 定义最大瞬态存储大小类型为ConstU32<{ 1024 * 1024 }>
    type MaxTransientStorageSize = ConstU32<{ 1024 * 1024 }>;
    // 定义最大调试缓冲区长度类型为ConstU32<{ 2 * 1024 * 1024 }>
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
    // 定义不安全不稳定接口类型为ConstBool<true>
    type UnsafeUnstableInterface = ConstBool<true>;
    // 定义代码哈希锁定存款百分比类型为CodeHashLockupDepositPercent
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    // 定义最大委托依赖类型为MaxDelegateDependencies
    type MaxDelegateDependencies = MaxDelegateDependencies;
    // 定义运行时持有原因类型为RuntimeHoldReason
    type RuntimeHoldReason = RuntimeHoldReason;
    
    // 定义环境类型为()
    type Environment = ();
    // 定义调试类型为()
    type Debug = ();
    // 定义API版本类型为()
    type ApiVersion = ();
    // 定义迁移类型为()
    type Migrations = ();
    // 根据特征配置Xcm类型
    #[cfg(feature = "parachain")]
    type Xcm = pallet_xcm::Pallet<Self>;
    #[cfg(not(feature = "parachain"))]
    type Xcm = ();
    
    // 定义上传来源类型为EnsureSigned<Self::AccountId>
    type UploadOrigin = EnsureSigned<Self::AccountId>;
    // 定义实例化来源类型为EnsureSigned<Self::AccountId>
    type InstantiateOrigin = EnsureSigned<Self::AccountId>;
}