use zksync_web3_decl::client::{Client, L1};
use zksync_eth_client::EthInterface;
use zksync_basic_types::web3::{Log, Filter, BlockNumber, FilterBuilder, CallRequest, BlockId};
use zksync_basic_types::ethabi::{Contract, Event, ParamType, RawTopicFilter};
use zksync_basic_types::ethabi::decode;
use zksync_basic_types::{U256, H256};
use std::fs::File;
use ethereum_types::U64;
use serde_json;
use serde::Deserialize;
use base64;

mod tendermint_client;

mod blobstream;
use blobstream::{find_block_range, DataRootInclusionProofResponse, get_latest_block};

#[tokio::main]
async fn main() {
    let contract = Contract::load(File::open("blobstream.json").unwrap()).unwrap();
    let blobstream_update_event = contract.events_by_name("DataCommitmentStored").unwrap()[0].clone();
    println!("event by name: {:?}", blobstream_update_event);

    let client: Client<L1> = Client::http("https://eth-sepolia.g.alchemy.com/v2/nCakZRn9VQg2I-CWYm6hVKpM4pvBYLWg".parse().unwrap())
        .expect("Could not create client")
        .build();
    let block_num = client.block_number()
        .await
        .expect("Could not get block number");

    let latest_block = get_latest_block(&client, &contract).await;
    println!("Latest blobstream block: {}", latest_block);

    let target_height: u64 = 4774355;

    let (from, to) = find_block_range(
        &client,
        target_height,
        latest_block,
        BlockNumber::Number(block_num),
        &blobstream_update_event,
        &contract
    ).await.expect("Failed to find block range");

    println!("From: {}, To: {}", from, to);

    // TODO: 
    // Get the inclusion proof with these docs https://docs.celestia.org/how-to-guides/blobstream-proof-queries
    let tm_rpc_client = tendermint_client::TendermintRPCClient::new("http://public-celestia-mocha4-consensus.numia.xyz:26657".to_string());
    let proof = tm_rpc_client.get_data_root_inclusion_proof(
        from.as_u64()+(to.as_u64() - from.as_u64())/2, 
        from.as_u64(), 
        to.as_u64()
    ).await.unwrap();
    
    let data_root_inclusion_proof_response: DataRootInclusionProofResponse = serde_json::from_str(&proof).unwrap();
    let data_root_inclusion_proof = data_root_inclusion_proof_response.result.proof;
    for aunt in data_root_inclusion_proof.aunts {
        println!("{}", hex::encode(aunt));
    }
}
