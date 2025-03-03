use std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	BuildSpec(sc_cli::BuildSpecCmd),

	CheckBlock(sc_cli::CheckBlockCmd),

	ExportBlocks(sc_cli::ExportBlocksCmd),

	ExportState(sc_cli::ExportStateCmd),

	ImportBlocks(sc_cli::ImportBlocksCmd),

	Revert(sc_cli::RevertCmd),

	PurgeChain(cumulus_client_cli::PurgeChainCmd),

	#[command(alias = "export-genesis-state")]
	ExportGenesisHead(cumulus_client_cli::ExportGenesisHeadCommand),

	ExportGenesisWasm(cumulus_client_cli::ExportGenesisWasmCommand),

	#[command(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}

const AFTER_HELP_EXAMPLE: &str = color_print::cstr!(
	r#"<bold><underline>Examples:</></>
   <bold>parachain-contracts-node build-spec --disable-default-bootnode > plain-parachain-chainspec.json</>
           Export a chainspec for a local testnet in json format.
   <bold>parachain-contracts-node --chain plain-parachain-chainspec.json --tmp -- --chain rococo-local</>
           Launch a full node with chain specification loaded from plain-parachain-chainspec.json.
   <bold>parachain-contracts-node</>
           Launch a full node with default parachain <italic>local-testnet</> and relay chain <italic>rococo-local</>.
   <bold>parachain-contracts-node --collator</>
           Launch a collator with default parachain <italic>local-testnet</> and relay chain <italic>rococo-local</>.
 "#
);
#[derive(Debug, clap::Parser)]
#[command(
	propagate_version = true,
	args_conflicts_with_subcommands = true,
	subcommand_negates_reqs = true
)]
#[clap(after_help = AFTER_HELP_EXAMPLE)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[command(flatten)]
	pub run: cumulus_client_cli::RunCmd,

	#[arg(long)]
	pub no_hardware_benchmarks: bool,

	#[arg(raw = true)]
	pub relay_chain_args: Vec<String>,

	#[arg(long, default_value_t = 1)]
	pub finalize_delay_sec: u8,
}

#[derive(Debug)]
pub struct RelayChainCli {
	pub base: polkadot_cli::RunCmd,

	pub chain_id: Option<String>,

	pub base_path: Option<PathBuf>,
}

impl RelayChainCli {
	pub fn new<'a>(
		para_config: &sc_service::Configuration,
		relay_chain_args: impl Iterator<Item = &'a String>,
	) -> Self {
		let extension = crate::chain_spec::Extensions::try_get(&*para_config.chain_spec);
		let chain_id = extension.map(|e| e.relay_chain.clone());
		let base_path = para_config.base_path.path().join("polkadot");
		Self {
			base_path: Some(base_path),
			chain_id,
			base: clap::Parser::parse_from(relay_chain_args),
		}
	}
}
