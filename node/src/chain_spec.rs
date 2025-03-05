pub mod dev;

use contracts_parachain_runtime::{AccountId, AuraId, Signature, EXISTENTIAL_DEPOSIT};
use cumulus_primitives_core::ParaId;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

// 定义链规范类型的别名
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

// 定义安全的XCM版本常量
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// 从种子字符串获取公共钥匙
/// 
/// # 参数
/// 
/// * `seed`: 种子字符串
/// 
/// # 返回值
/// 
/// 返回指定公共钥匙类型`TPublic`的实例
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// 定义链规范的扩展属性
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	#[serde(alias = "relayChain", alias = "RelayChain")]
	pub relay_chain: String,
	#[serde(alias = "paraId", alias = "ParaId")]
	pub para_id: u32,
}

/// 从链规范中获取扩展属性
/// 
/// # 参数
/// 
/// * `chain_spec`: 链规范实例
/// 
/// # 返回值
/// 
/// 返回扩展属性的引用
impl Extensions {
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

// 定义账户公共钥匙类型的别名
type AccountPublic = <Signature as Verify>::Signer;

/// 从种子字符串获取收集者钥匙
/// 
/// # 参数
/// 
/// * `seed`: 种子字符串
/// 
/// # 返回值
/// 
/// 返回`AuraId`类型的实例
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	get_from_seed::<AuraId>(seed)
}

/// 从种子字符串获取账户ID
/// 
/// # 参数
/// 
/// * `seed`: 种子字符串
/// 
/// # 返回值
/// 
/// 返回`AccountId`类型的实例
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// 创建模板会话钥匙
/// 
/// # 参数
/// 
/// * `keys`: Aura钥匙
/// 
/// # 返回值
/// 
/// 返回包含Aura钥匙的`SessionKeys`实例
pub fn template_session_keys(keys: AuraId) -> contracts_parachain_runtime::SessionKeys {
	contracts_parachain_runtime::SessionKeys { aura: keys }
}

/// 生成本地测试网配置
/// 
/// # 返回值
/// 
/// 返回`ChainSpec`实例，配置了本地测试网
pub fn local_testnet_config() -> ChainSpec {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "UNIT".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("ss58Format".into(), 42.into());

	#[allow(deprecated)]
	ChainSpec::builder(
		contracts_parachain_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "rococo-local".into(), para_id: 1000 },
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(testnet_genesis(
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_collator_keys_from_seed("Alice"),
			),
			(
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_collator_keys_from_seed("Bob"),
			),
		],
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		1000.into(),
	))
	.with_protocol_id("contracts-local")
	.with_properties(properties)
	.build()
}

/// 生成测试网创世配置
/// 
/// # 参数
/// 
/// * `invulnerables`: 收集者账户和钥匙对的向量
/// * `endowed_accounts`: 被赋予初始余额的账户向量
/// * `root`: 根账户
/// * `id`: Parachain ID
/// 
/// # 返回值
/// 
/// 返回一个JSON值，表示创世配置
fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	root: AccountId,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 1u64 << 60)).collect::<Vec<_>>(),
		},
		"parachainInfo": {
			"parachainId": id,
		},
		"collatorSelection": {
			"invulnerables": invulnerables.iter().cloned().map(|(acc, _)| acc).collect::<Vec<_>>(),
			"candidacyBond": EXISTENTIAL_DEPOSIT * 16,
		},
		"session": {
			"keys": invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),
						acc,
						template_session_keys(aura),
					)
				})
			.collect::<Vec<_>>(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"sudo": { "key": Some(root) }
	})
}