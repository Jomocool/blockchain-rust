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
	pub const TokenLocation: Location = Here.into_location();
	pub const RelayLocation: Location = Location::parent();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

pub type LocationToAccountId =
	(AccountId32Aliases<RelayNetwork, AccountId>, ParentIsPreset<AccountId>);

#[allow(deprecated)]
pub type LocalAssetTransactor =
	CurrencyAdapter<Balances, IsConcrete<RelayLocation>, LocationToAccountId, AccountId, ()>;

#[allow(deprecated)]
pub type LocalBalancesTransactor =
	CurrencyAdapter<Balances, IsConcrete<TokenLocation>, LocationToAccountId, AccountId, ()>;

pub type AssetTransactors = (LocalBalancesTransactor, LocalAssetTransactor);

pub type XcmOriginToTransactDispatchOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
		pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
	}
}

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

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = AssetTransactors;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader =
		UsingComponents<WeightToFee, RelayLocation, AccountId, Balances, ToAuthor<Runtime>>;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = PolkadotXcm;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

pub type XcmRouter =
	WithUniqueTopic<(cumulus_primitives_utility::ParentAsUmp<ParachainSystem, (), ()>, XcmpQueue)>;

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<Location> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Nothing;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}
