use zksync_web3_decl::client::{Client, L1};
use zksync_eth_client::EthInterface;
use zksync_basic_types::web3::{Log, Filter, BlockNumber, FilterBuilder};
use zksync_basic_types::ethabi::{Contract, Event, ParamType, RawTopicFilter};
use zksync_basic_types::ethabi::decode;
use zksync_basic_types::{U256, H256};
use std::fs::File;

mod tm_rpc_utils;

#[derive(Debug)]
pub struct DataCommitmentStored {
    pub proof_nonce: U256,
    pub start_block: U256,
    pub end_block: U256,
    pub data_commitment: H256,
}

impl DataCommitmentStored {
    pub fn from_log(log: &Log) -> Self {
        DataCommitmentStored {
            proof_nonce: decode(&[ParamType::Uint(256)], &log.data.0)
                .unwrap()[0]
                .clone()
                .into_uint()
                .unwrap(),
            start_block: U256::from_big_endian(&log.topics[1].as_bytes()),
            end_block: U256::from_big_endian(&log.topics[2].as_bytes()),
            data_commitment: H256::from_slice(&log.topics[3].as_bytes()),
        }
    }
}


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
    println!("Block number: {}", block_num);

    println!("Event signature: {:?}", blobstream_update_event.signature());
    
    let filter = FilterBuilder::default()
        .from_block(BlockNumber::Number(block_num-500))
        .to_block(BlockNumber::Number(block_num))
        .address(vec!["0xF0c6429ebAB2e7DC6e05DaFB61128bE21f13cb1e".parse().unwrap()])
        .topics(
            Some(vec![blobstream_update_event.signature()]),
            None,
            None,
            None
        )
        .build();
    let logs = client.logs(&filter).await.expect("Could not get logs");
    let log = logs[0].clone();
    println!("Logs: {:?}", logs);
    let data_commitment_stored = DataCommitmentStored::from_log(&log);
    println!("Data commitment stored: {:?}", data_commitment_stored);

    // TODO: 
    // Get the inclusion proof with these docs https://docs.celestia.org/how-to-guides/blobstream-proof-queries
    let tm_rpc_client = tm_rpc_utils::TendermintRPCClient::new("http://public-celestia-mocha4-consensus.numia.xyz:26657".to_string());
    let proof = tm_rpc_client.get_data_root_inclusion_proof(data_commitment_stored.start_block.as_u64()+(data_commitment_stored.end_block.as_u64() - data_commitment_stored.start_block.as_u64())/2, data_commitment_stored.start_block.as_u64(), data_commitment_stored.end_block.as_u64() ).await.unwrap();
    println!("Proof: {:?}", proof);
}
