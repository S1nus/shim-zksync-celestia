use zksync_web3_decl::client::{Client, L1};
use zksync_eth_client::EthInterface;
use zksync_basic_types::web3::{Log, Filter, BlockNumber, FilterBuilder, CallRequest, BlockId};
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

async fn get_latest_block(client: &Client<L1>, contract: &Contract) -> U256 {
    let request = CallRequest {
        to: Some("0xF0c6429ebAB2e7DC6e05DaFB61128bE21f13cb1e".parse().unwrap()),
        data: Some(contract.function("latestBlock").unwrap().encode_input(&[]).unwrap().into()),
        ..Default::default()
    };
    let block_num = client.block_number().await.expect("Could not get block number");
    let result = client.call_contract_function(request, Some(BlockId::Number(block_num.into()))).await.unwrap().0;
    decode(&[ParamType::Uint(256)], &result).unwrap()[0].clone().into_uint().unwrap()
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

    let mut latest_block = get_latest_block(&client, &contract).await;
    println!("Latest blobstream block: {}", latest_block);

    let target_height: u64 = 4774355;

    let mut from: Option<U256> = None;
    let mut to: Option<U256> = None;
    if target_height < latest_block.as_u64() {
        println!("Target height is less than latest block, searching events...");
        let mut multiplier = 1;
        let mut page_start = block_num;
        let mut page_end = page_start - 500 * multiplier;
        let mut found = false;
        while !found {
            let filter = FilterBuilder::default()
                .from_block(BlockNumber::Number(page_end))
                .to_block(BlockNumber::Number(page_start))
                .address(vec!["0xF0c6429ebAB2e7DC6e05DaFB61128bE21f13cb1e".parse().unwrap()])
                .topics(
                    Some(vec![blobstream_update_event.signature()]),
                    None,
                    None,
                    None
                )
                .build();
            let logs = client.logs(&filter).await.expect("Could not get logs");
            for log in logs {
                println!("checking a log...");
                let data_commitment_stored = DataCommitmentStored::from_log(&log);
                if data_commitment_stored.start_block.as_u64() <= target_height && data_commitment_stored.end_block.as_u64() > target_height {
                    println!("Found the log!");
                    found = true;
                    from = Some(data_commitment_stored.start_block);
                    to = Some(data_commitment_stored.end_block);
                    break;
                }
            }
            if !found {
                println!("No log found, searching in the next page...");
                multiplier += 1;
                page_start = page_end;
                page_end = page_start - 500 * multiplier;
            }
        }
    } else {
        println!("Target height is greater than latest block, waiting for next update...");
        from = Some(latest_block);
        while (latest_block < target_height.into()) {
            latest_block = get_latest_block(&client, &contract).await;
            println!("Latest blobstream block: {}", latest_block);
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        to = Some(latest_block);
    }

    println!("From: {:?}, To: {:?}", from, to);

    // TODO: 
    // Get the inclusion proof with these docs https://docs.celestia.org/how-to-guides/blobstream-proof-queries
    let tm_rpc_client = tm_rpc_utils::TendermintRPCClient::new("http://public-celestia-mocha4-consensus.numia.xyz:26657".to_string());
    let proof = tm_rpc_client.get_data_root_inclusion_proof(from.unwrap().as_u64()+(to.unwrap().as_u64() - from.unwrap().as_u64())/2, from.unwrap().as_u64(), to.unwrap().as_u64() ).await.unwrap();
    println!("Proof: {:?}", proof);
}
