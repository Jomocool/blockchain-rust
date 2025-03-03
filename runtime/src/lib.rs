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

pub type BlockNumber = u32;

pub type Signature = MultiSignature;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub type Balance = u128;

pub type Nonce = u32;

pub type Hash = sp_core::H256;

pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {}
	}
}
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("substrate-contracts-node"),
	impl_name: create_runtime_str!("substrate-contracts-node"),
	authoring_version: 1,
	spec_version: 100,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

const CONTRACTS_DEBUG_OUTPUT: pallet_contracts::DebugInfo =
	pallet_contracts::DebugInfo::UnsafeDebug;
const CONTRACTS_EVENTS: pallet_contracts::CollectEvents =
	pallet_contracts::CollectEvents::UnsafeCollect;
const REVIVE_DEBUG_OUTPUT: pallet_revive::DebugInfo = pallet_revive::DebugInfo::UnsafeDebug;
const REVIVE_EVENTS: pallet_revive::CollectEvents = pallet_revive::CollectEvents::UnsafeCollect;

const MILLIUNIT: Balance = 1_000_000_000;
pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;

					pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
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

	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	type Block = Block;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type AccountId = AccountId;
	type Nonce = Nonce;
	type Hash = Hash;
	type BlockHashCount = BlockHashCount;
	type DbWeight = RocksDbWeight;
	type Version = Version;
	type AccountData = pallet_balances::AccountData<Balance>;
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = ();
	type EventHandler = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

#[frame_support::runtime]
mod runtime {
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
	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type RandomnessCollectiveFlip = pallet_insecure_randomness_collective_flip;
	#[runtime::pallet_index(2)]
	pub type Utility = pallet_utility;
	#[runtime::pallet_index(3)]
	pub type Timestamp = pallet_timestamp;
	#[runtime::pallet_index(4)]
	pub type Balances = pallet_balances;
	#[runtime::pallet_index(5)]
	pub type Authorship = pallet_authorship;
	#[runtime::pallet_index(6)]
	pub type TransactionPayment = pallet_transaction_payment;
	#[runtime::pallet_index(7)]
	pub type Sudo = pallet_sudo;
	#[runtime::pallet_index(8)]
	pub type Contracts = pallet_contracts;
	#[runtime::pallet_index(9)]
	pub type Revive = pallet_revive;
	#[runtime::pallet_index(10)]
	pub type Assets = pallet_assets;
}

pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
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
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

type EventRecord = frame_system::EventRecord<
	<Runtime as frame_system::Config>::RuntimeEvent,
	<Runtime as frame_system::Config>::Hash,
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall> for Runtime {
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord>
		for Runtime
	{
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

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
			determinism: pallet_contracts::Determinism,
		) -> pallet_contracts::CodeUploadResult<Hash, Balance>
		{
			Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
		}

		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pallet_contracts::GetStorageResult {
			Contracts::get_storage(address, key)
		}
	}

	impl pallet_revive::ReviveApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord> for Runtime
	{
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
}
