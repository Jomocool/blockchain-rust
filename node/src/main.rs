mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;
mod rpc;

/// 程序入口
fn main() -> sc_cli::Result<()> {
	command::run()
}
