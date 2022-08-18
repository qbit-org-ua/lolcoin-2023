use dotenv::dotenv;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

use near_lake_framework::near_indexer_primitives;

pub(crate) const INDEXER: &str = "indexer_lolcoin";

mod integers;
use self::integers::U128;

use once_cell::sync::OnceCell;

static USERS_TABLE: OnceCell<std::sync::Mutex<Vec<User>>> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    full_name: String,
    school_grade: String,
    account_id: String,
    balance: U128,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    init_tracing();

    let start_block_height = std::fs::read_to_string("last_block.txt")
        .as_deref()
        .unwrap_or("")
        .trim()
        .parse()
        .unwrap_or(97362869)
        + 1;
    USERS_TABLE.get_or_init(|| {
        let users_table_file = std::fs::File::open("data.json").expect("could not read data.json");
        let users_table_reader = std::io::BufReader::new(users_table_file);
        std::sync::Mutex::new(
            serde_json::from_reader(users_table_reader).expect("could not parse data.json"),
        )
    });

    let config = near_lake_framework::LakeConfigBuilder::default()
        .testnet()
        .start_block_height(start_block_height)
        .blocks_preload_pool_size(10)
        .build()?;

    let (lake_handle, stream) = near_lake_framework::streamer(config);

    let mut handlers = tokio_stream::wrappers::ReceiverStream::new(stream)
        .map(|streamer_message| handle_streamer_message(streamer_message))
        .buffer_unordered(1usize);

    // let mut time_now = std::time::Instant::now();
    while let Some(handle_message) = handlers.next().await {
        match handle_message {
            Ok(_) => {
                // let elapsed = time_now.elapsed();
                // println!(
                //     "Elapsed time spent on block {}: {:.3?}",
                //     block_height, elapsed
                // );
                // time_now = std::time::Instant::now();
            }
            Err(e) => {
                return Err(anyhow::anyhow!(e));
            }
        }
    }

    // propagate errors from the Lake Framework
    match lake_handle.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(anyhow::Error::from(e)), // JoinError
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "standard")]
#[serde(rename_all = "snake_case")]
pub(crate) enum NearEvent {
    Nep141(Nep141Event),
}

// *** NEP-141 FT ***
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Nep141Event {
    pub version: String,
    #[serde(flatten)]
    pub event_kind: Nep141EventKind,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Nep141EventKind {
    FtMint(Vec<FtMintData>),
    FtTransfer(Vec<FtTransferData>),
    FtBurn(Vec<FtBurnData>),
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct FtMintData {
    pub owner_id: String,
    pub amount: U128,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct FtTransferData {
    pub old_owner_id: String,
    pub new_owner_id: String,
    pub amount: U128,
    pub memo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct FtBurnData {
    pub owner_id: String,
    pub amount: U128,
    pub memo: Option<String>,
}

pub(crate) fn extract_events(
    outcome: &near_indexer_primitives::IndexerExecutionOutcomeWithReceipt,
) -> Vec<Nep141EventKind> {
    let prefix = "EVENT_JSON:";
    outcome
        .execution_outcome
        .outcome
        .logs
        .iter()
        .filter_map(|untrimmed_log| {
            let log = untrimmed_log.trim();
            if !log.starts_with(prefix) {
                return None;
            }

            match serde_json::from_str::<'_, NearEvent>(log[prefix.len()..].trim()) {
                Ok(result) => Some(result),
                Err(err) => None,
            }
        })
        .map(|event| {
            let NearEvent::Nep141(ft_event) = event;
            ft_event.event_kind
        })
        .collect()
}

async fn handle_streamer_message(
    streamer_message: near_indexer_primitives::StreamerMessage,
) -> anyhow::Result<()> {
    //    if streamer_message.block.header.height % 100 == 0 {
    tracing::info!(
        target: crate::INDEXER,
        "{} / shards {}",
        streamer_message.block.header.height,
        streamer_message.shards.len()
    );
    //    }

    let block_height = streamer_message.block.header.height;
    let lolcoin_events: Vec<Nep141EventKind> = streamer_message
        .shards
        .into_iter()
        .flat_map(|shard| {
            shard
                .receipt_execution_outcomes
                .into_iter()
                .filter(|execution_outcome| {
                    execution_outcome.receipt.receiver_id
                        == "dev-1660278675045-43011334123486".parse().unwrap()
                })
                .flat_map(|execution_outcome| extract_events(&execution_outcome))
        })
        .collect();
    //tracing::info!(target: crate::INDEXER, "EVENTS: {:?}", lolcoin_events);

    if !lolcoin_events.is_empty() {
        tracing::info!(target: crate::INDEXER, "EVENTS: {:?}", lolcoin_events);
        //tracing::info!(target: crate::INDEXER, "not empty");
        for event in lolcoin_events {
            match event {
                Nep141EventKind::FtMint(mints) => {
                    for mint in mints {
                        update_user_balance(&mint.owner_id, mint.amount, Update::Deposit);
                    }
                }
                Nep141EventKind::FtBurn(burns) => {
                    for burn in burns {
                        update_user_balance(&burn.owner_id, burn.amount, Update::Withdraw);
                    }
                }
                Nep141EventKind::FtTransfer(transfers) => {
                    for transfer in transfers {
                        update_user_balance(
                            &transfer.old_owner_id,
                            transfer.amount,
                            Update::Withdraw,
                        );
                        update_user_balance(
                            &transfer.new_owner_id,
                            transfer.amount,
                            Update::Deposit,
                        );
                    }
                }
            }
        }
        let users_table_file = std::fs::File::create("data.json").unwrap();
        let users_table: std::sync::MutexGuard<Vec<User>> =
            USERS_TABLE.get().unwrap().lock().unwrap();
        serde_json::to_writer_pretty(users_table_file, &*users_table).unwrap();
    }

    tokio::fs::write("last_block.txt", block_height.to_string()).await?;
    Ok(())
}

enum Update {
    Deposit,
    Withdraw,
}

fn update_user_balance(account_id: &str, amount: U128, update: Update) {
    eprintln!("UPDATE: {}", account_id);
    let mut users_table_local = USERS_TABLE.get().unwrap().lock().unwrap();
    // It is inefficient, but will be more than good enough given the load
    for user in users_table_local.iter_mut() {
        if user.account_id == account_id {
            match update {
                Update::Deposit => {
                    user.balance = user.balance
                        .0
                        .checked_add(amount.0)
                        .expect("we hit overflow")
                        .into();
                }
                Update::Withdraw => {
                    user.balance = user.balance
                        .0
                        .checked_sub(amount.0)
                        .expect("we hit underflow")
                        .into();
                }
            }
            return;
        }
    }
    eprintln!("Unknown user {}", account_id);
    match update {
        Update::Deposit => {
            users_table_local.push(User {
                account_id: account_id.to_string(),
                balance: amount,
                full_name: account_id.to_string(),
                school_grade: "".to_string(),
            });
        }
        Update::Withdraw => {
            panic!("Unknown user lost tokens which leads to underflow");
        }
    }
}

fn init_tracing() {
    let mut env_filter = EnvFilter::new("near_lake_framework=info,indexer_lolcoin=info");

    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        if !rust_log.is_empty() {
            for directive in rust_log.split(',').filter_map(|s| match s.parse() {
                Ok(directive) => Some(directive),
                Err(err) => {
                    tracing::warn!(
                        target: crate::INDEXER,
                        "Ignoring directive `{}`: {}",
                        s,
                        err
                    );
                    None
                }
            }) {
                env_filter = env_filter.add_directive(directive);
            }
        }
    }

    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
}
