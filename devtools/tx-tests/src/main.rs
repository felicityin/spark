use common::logger;
use common::types::tx_builder::NetworkType;
use config::types::PrivKeys;
use rpc_client::ckb_client::ckb_rpc_client::CkbRpcClient;
use tx_builder::ckb::helper::OmniEth;
use tx_builder::set_network_type;

use crate::cases::*;
use crate::config::{parse_log_config, parse_priv_keys};
use crate::tx::*;

mod cases;
mod config;
mod mock;
mod tx;

pub const PRIV_KEYS_PATH: &str = "./src/config/priv_keys.toml";
pub const TYPE_IDS_PATH: &str = "./src/config/type_ids.toml";
pub const LOG_CONFIG_PATH: &str = "./src/config/log.toml";
pub const ROCKSDB_PATH: &str = "./free-space/smt";

#[tokio::main]
async fn main() {
    let cmd = clap::Command::new("spark")
        .version(clap::crate_version!())
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("cases")
                .about("Test cases")
                .arg(
                    clap::Arg::new("net")
                        .short('n')
                        .required(false)
                        .num_args(1)
                        .value_parser(["dev", "test", "main"])
                        .default_value("test")
                        .help("Switch network"),
                )
                .arg(
                    clap::Arg::new("case1")
                        .short('1')
                        .required(false)
                        .num_args(0)
                        .help("Test case 1"),
                ),
        )
        .subcommand(
            clap::Command::new("single-tx")
                .about("Test single tx")
                .arg(
                    clap::Arg::new("net")
                        .short('n')
                        .required(false)
                        .num_args(1)
                        .value_parser(["dev", "test", "main"])
                        .default_value("test")
                        .help("Switch network"),
                )
                .arg(
                    clap::Arg::new("faucet")
                        .short('f')
                        .required(false)
                        .num_args(0)
                        .help("Send CKB from secp256k1 address to Omni ETH CKB address"),
                )
                .arg(
                    clap::Arg::new("init")
                        .short('i')
                        .required(false)
                        .num_args(0)
                        .help("Test init tx"),
                )
                .arg(
                    clap::Arg::new("mint")
                        .short('m')
                        .required(false)
                        .num_args(0)
                        .help("Test mint tx"),
                )
                .arg(
                    clap::Arg::new("stake")
                        .short('s')
                        .required(false)
                        .num_args(1)
                        .value_parser(["first", "add", "redeem"])
                        .help("Test stake tx"),
                )
                .arg(
                    clap::Arg::new("delegate")
                        .short('d')
                        .required(false)
                        .num_args(1)
                        .value_parser(["first", "add", "redeem"])
                        .help("Test delegate tx"),
                )
                .arg(
                    clap::Arg::new("checkpoint")
                        .short('c')
                        .required(false)
                        .num_args(0)
                        .help("Test checkpoint tx"),
                )
                .arg(
                    clap::Arg::new("stake-smt")
                        .short('t')
                        .required(false)
                        .num_args(0)
                        .help("Test stake smt tx"),
                )
                .arg(
                    clap::Arg::new("delegate-smt")
                        .short('e')
                        .required(false)
                        .num_args(0)
                        .help("Test delegate smt tx"),
                )
                .arg(
                    clap::Arg::new("withdraw")
                        .short('w')
                        .required(false)
                        .num_args(0)
                        .help("Test withdraw tx"),
                )
                .arg(
                    clap::Arg::new("metadata")
                        .short('a')
                        .required(false)
                        .num_args(0)
                        .help("Test metadata tx"),
                )
                .arg(
                    clap::Arg::new("reward")
                        .short('r')
                        .required(false)
                        .num_args(0)
                        .help("Test reward tx"),
                ),
        );

    register_log();

    let priv_keys = parse_priv_keys(PRIV_KEYS_PATH);

    let matches = cmd.get_matches();
    match matches.subcommand() {
        Some(("cases", matches)) => run_test_cases(matches, priv_keys).await,
        Some(("single-tx", matches)) => run_single_tx(matches, priv_keys).await,
        _ => unimplemented!(),
    }
}

async fn run_test_cases(matches: &clap::ArgMatches, priv_keys: PrivKeys) {
    let net = matches.get_one::<String>("net").unwrap().as_str();
    let case1 = matches.get_one::<bool>("case1").unwrap();

    let ckb = parse_ckb_net(net);

    if *case1 {
        run_case1(&ckb, priv_keys).await;
    }
}

async fn run_single_tx(matches: &clap::ArgMatches, priv_keys: PrivKeys) {
    let net = matches.get_one::<String>("net").unwrap().as_str();
    let faucet = *matches.get_one::<bool>("faucet").unwrap();
    let init = *matches.get_one::<bool>("init").unwrap();
    let mint = *matches.get_one::<bool>("mint").unwrap();
    let stake = matches.get_one::<String>("stake");
    let delegate = matches.get_one::<String>("delegate");
    let checkpoint = *matches.get_one::<bool>("checkpoint").unwrap();
    let stake_smt = *matches.get_one::<bool>("stake-smt").unwrap();
    let delegate_smt = *matches.get_one::<bool>("delegate-smt").unwrap();
    let withdraw = *matches.get_one::<bool>("withdraw").unwrap();
    let metadata = *matches.get_one::<bool>("metadata").unwrap();
    let reward = *matches.get_one::<bool>("reward").unwrap();

    let ckb = parse_ckb_net(net);

    let kicker_key = priv_keys.staker_privkeys[0].clone().into_h256().unwrap();
    let staker_key = priv_keys.staker_privkeys[0].clone().into_h256().unwrap();
    let delegator_key = priv_keys.delegator_privkeys[0].clone().into_h256().unwrap();
    let staker_eth_addr = OmniEth::new(staker_key.clone()).address().unwrap();

    if faucet {
        run_faucet_tx(&ckb, priv_keys.clone()).await;
    } else if init {
        run_init_tx(&ckb, priv_keys.clone()).await;
    } else if mint {
        run_mint_tx(&ckb, priv_keys.clone().clone()).await;
    } else if stake.is_some() {
        match stake.unwrap().as_str() {
            "first" => first_stake_tx(&ckb, staker_key).await,
            "add" => add_stake_tx(&ckb, staker_key).await,
            "redeem" => reedem_stake_tx(&ckb, staker_key).await,
            _ => unimplemented!(),
        }
    } else if delegate.is_some() {
        match delegate.unwrap().as_str() {
            "first" => first_delegate_tx(&ckb, delegator_key, staker_eth_addr).await,
            "add" => add_delegate_tx(&ckb, delegator_key, staker_eth_addr).await,
            "redeem" => reedem_delegate_tx(&ckb, delegator_key, staker_eth_addr).await,
            _ => unimplemented!(),
        }
    } else if checkpoint {
        run_checkpoint_tx(&ckb, priv_keys, 1).await;
    } else if stake_smt {
        run_stake_smt_tx(&ckb, kicker_key).await;
    } else if delegate_smt {
        run_delegate_smt_tx(&ckb, kicker_key).await;
    } else if withdraw {
        let user_key = priv_keys.staker_privkeys[0].clone().into_h256().unwrap();
        run_withdraw_tx(&ckb, user_key).await;
    } else if metadata {
        run_metadata_tx(&ckb, kicker_key).await;
    } else if reward {
        let user_key = priv_keys.staker_privkeys[0].clone().into_h256().unwrap();
        run_reward_tx(&ckb, user_key).await;
    } else {
        unimplemented!();
    }
}

fn register_log() {
    let config = parse_log_config(LOG_CONFIG_PATH);

    logger::init(
        config.filter.clone(),
        config.log_to_console,
        config.console_show_file_and_line,
        config.log_to_file,
        config.log_path.clone(),
        config.file_size_limit,
    );
}

fn parse_ckb_net(net: &str) -> CkbRpcClient {
    match net {
        "dev" => {
            println!("dev net");
            set_network_type(NetworkType::Devnet);
            CkbRpcClient::new("http://127.0.0.1:8114")
        }
        "test" => {
            println!("test net");
            set_network_type(NetworkType::Testnet);
            CkbRpcClient::new("https://testnet.ckb.dev")
        }
        "main" => {
            println!("main net");
            set_network_type(NetworkType::Mainnet);
            CkbRpcClient::new("https://mainnet.ckb.dev")
        }
        _ => unimplemented!(),
    }
}
