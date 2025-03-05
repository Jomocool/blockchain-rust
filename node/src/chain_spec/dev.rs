use contracts_node_runtime::{AccountId, Signature, WASM_BINARY};
use sc_service::ChainType;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// 定义链配置的类型
pub type ChainSpec = sc_service::GenericChainSpec;

/// 从给定的种子字符串中生成一个公共钥匙
/// 
/// # 参数
/// 
/// * `seed`: 用于生成钥匙的种子字符串
/// 
/// # 返回值
/// 
/// 返回从种子生成的公共钥匙
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// 定义账户公共钥匙的类型
type AccountPublic = <Signature as Verify>::Signer;

/// 从给定的种子字符串中生成一个账户ID
/// 
/// # 参数
/// 
/// * `seed`: 用于生成账户ID的种子字符串
/// 
/// # 返回值
/// 
/// 返回从种子生成的账户ID
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// 生成开发环境的链配置
/// 
/// # 返回值
/// 
/// 返回一个构建好的链配置，或者一个错误附带错误信息
pub fn development_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(WASM_BINARY.expect("Development wasm not available"), Default::default())
		.with_name("Development")
		.with_id("dev")
		.with_chain_type(ChainType::Development)
		.with_genesis_config_patch(testnet_genesis(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
				get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				get_account_id_from_seed::<sr25519::Public>("Dave"),
				get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
				get_account_id_from_seed::<sr25519::Public>("Eve"),
				get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie"),
				get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
			],
			true,
		))
		.build())
}

/// 生成测试网络的创世配置
/// 
/// # 参数
/// 
/// * `root_key`: 用于配置的根密钥
/// * `endowed_accounts`: 一组预配置的账户ID
/// * `_enable_println`: 一个标志，指示是否启用打印
/// 
/// # 返回值
/// 
/// 返回一个包含创世配置的JSON值
fn testnet_genesis(
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> serde_json::Value {
	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 1u64 << 60)).collect::<Vec<_>>(),
		},
		"sudo": {
			"key": Some(root_key),
		},
	})
}