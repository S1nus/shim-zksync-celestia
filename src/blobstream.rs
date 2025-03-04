use zksync_web3_decl::client::{Client, L1};
use zksync_eth_client::EthInterface;
use zksync_basic_types::web3::{Log, BlockNumber, FilterBuilder, CallRequest, BlockId};
use zksync_basic_types::ethabi::{Contract, Event, ParamType};
use zksync_basic_types::ethabi::decode;
use zksync_basic_types::{U256, H256};
use serde::Deserialize;
use base64;

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

pub async fn get_latest_block(client: &Client<L1>, contract: &Contract) -> U256 {
    let request = CallRequest {
        to: Some("0xF0c6429ebAB2e7DC6e05DaFB61128bE21f13cb1e".parse().unwrap()),
        data: Some(contract.function("latestBlock").unwrap().encode_input(&[]).unwrap().into()),
        ..Default::default()
    };
    let block_num = client.block_number().await.expect("Could not get block number");
    let result = client.call_contract_function(request, Some(BlockId::Number(block_num.into()))).await.unwrap().0;
    decode(&[ParamType::Uint(256)], &result).unwrap()[0].clone().into_uint().unwrap()
}

pub async fn find_block_range(
    client: &Client<L1>,
    target_height: u64,
    latest_block: U256,
    eth_block_num: BlockNumber,
    blobstream_update_event: &Event,
    contract: &Contract,
) -> Result<(U256, U256), Box<dyn std::error::Error>> {
    if target_height < latest_block.as_u64() {
        // Search historical events
        println!("Target height is less than latest block, searching historical events");
        let mut page_start = match eth_block_num {
            BlockNumber::Number(num) => num,
            _ => return Err("Invalid block number".into()),
        };

        let contract_address = "0xF0c6429ebAB2e7DC6e05DaFB61128bE21f13cb1e".parse()?;

        for multiplier in 1.. {  // Infinite iterator with safety check
            if multiplier > 1000 {  // Safety limit to prevent infinite loops
                return Err("Exceeded maximum search depth".into());
            }

            let page_end = page_start - 500 * multiplier;
            let filter = FilterBuilder::default()
                .from_block(BlockNumber::Number(page_end))
                .to_block(BlockNumber::Number(page_start))
                .address(vec![contract_address])
                .topics(Some(vec![blobstream_update_event.signature()]), None, None, None)
                .build();

            let logs = client.logs(&filter).await?;
            
            if let Some(log) = logs.iter().find(|log| {
                let commitment = DataCommitmentStored::from_log(log);
                commitment.start_block.as_u64() <= target_height && 
                commitment.end_block.as_u64() > target_height
            }) {
                let commitment = DataCommitmentStored::from_log(log);
                return Ok((commitment.start_block, commitment.end_block));
            }

            page_start = page_end;
        }
        Err("No matching block range found".into())
    } else {
        // Wait for future blocks
        println!("Target height is greater than latest block, waiting for future updates");
        let mut current_block = latest_block;
        
        while current_block < target_height.into() {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            current_block = get_latest_block(client, &contract).await;
            println!("Latest blobstream block: {}", current_block);
        }
        
        Ok((latest_block, current_block))
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct DataRootInclusionProofResponse {
    pub jsonrpc: String,
    pub id: i64,
    pub result: DataRootInclusionProofResult,
}

#[derive(Debug, serde::Deserialize)]
pub struct DataRootInclusionProofResult {
    pub proof: DataRootInclusionProof,
}

#[derive(Debug, serde::Deserialize)]
pub struct DataRootInclusionProof {
    #[serde(deserialize_with = "deserialize_base64_vec")]
    pub aunts: Vec<Vec<u8>>,
    pub index: String,
    pub leaf_hash: String,
    pub total: String,
}

pub fn deserialize_base64_vec<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let strings: Vec<String> = Vec::deserialize(deserializer)?;
    strings
        .into_iter()
        .map(|s| base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).map_err(serde::de::Error::custom))
        .collect()
}