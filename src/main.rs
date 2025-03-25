use zksync_da_clients::celestia::CelestiaClient;
use zksync_config::configs::da_client::celestia::{CelestiaConfig, CelestiaSecrets};
use zksync_web3_decl::client::{Client, L1};
use zksync_eth_client::EthInterface;
use zksync_basic_types::web3::{Log, Filter, BlockNumber, FilterBuilder, CallRequest, BlockId};
use zksync_basic_types::ethabi::{Contract, Event, ParamType, RawTopicFilter};
use zksync_basic_types::ethabi::decode;
use zksync_basic_types::{U256, H256};
use zksync_da_client::DataAvailabilityClient;
use std::env;
use rand::RngCore;
use tracing_subscriber::{EnvFilter};
use std::io::Write;
use eq_sdk::BlobId;
use celestia_types::{blob::Commitment, block::Height as BlockHeight, nmt::Namespace, Height};
use base64::prelude::*;

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = CelestiaConfig {
        api_node_url: "https://grpc.archive.mocha.cumulo.com.es:443".to_string(),
        //eq_service_url: "https://eq-service-dev.eu-north-2.gateway.fm".to_string(),
        eq_service_grpc_url: "http://eqs.cnode.phd:50051".to_string(),
        namespace: "00000000000000000000000000000000000000000413528b469e1926".to_string(),
        //ychain_id: "2222-2".to_string(),
        chain_id: "mocha-4".to_string(),
        timeout_ms: 10000,
        celestia_core_tendermint_rpc_url: "http://public-celestia-mocha4-consensus.numia.xyz:26657".to_string(),
        blobstream_contract_address: "0xf0c6429ebab2e7dc6e05dafb61128be21f13cb1e".to_string(),
        num_pages: 500,
        page_size: 1000,
    };

    let secrets = CelestiaSecrets {
        private_key: env::var("PRIVATE_KEY")
            .expect("PRIVATE_KEY environment variable not set")
            .into()
    }; 

    let eth_client: Client<L1> = Client::http("https://eth-sepolia.g.alchemy.com/v2/nCakZRn9VQg2I-CWYm6hVKpM4pvBYLWg".parse().unwrap())
        .expect("Could not create client")
        .build();

    let da_client = CelestiaClient::new(config, secrets, Box::new(eth_client))
        .await
        .expect("Could not create DA client");
    
    println!("Blob size limit: {:?}", da_client.blob_size_limit());
    
    let blob_size = da_client.blob_size_limit().unwrap() / 2;
    
    let mut error_log = std::fs::File::create("error_log.txt")
        .expect("Failed to create error log file");

    let test_cases: Vec<(u32, &str, &str)> = vec![
        (5097912, "a00fc36d20187faa8a2e", "+3Pc84EgFrdj13uaW9nXV1xTe38Z2cAOYFBnlG6T4p0="),
        (5098217, "ca1de12ab8035a60aeec", "M+JtgYQzRMrZUyj8rW0nnkX29RSStdbeQfwXErw5V1Y="),
        (5098230, "ca1de12a1f4dbe943b6b", "lqdz37LujKKjMwpAfh/V17ZzgnIGwlhlmKtR+eIxpQ0="),
        (5098230, "ca1de12a86e37a25406c", "lEQTP9g4QhWCYgdccjxwXEjdcQNmxatt8z9Zo2KPp4o="),
        (5098230, "ca1de12a842f52467e34", "eOx6FZaVXF20fMGsOy1f8PJ4DODkxMa+GQBtfZg7l3s="),
        (5098245, "ca1de12ac5a629c3c42f", "nu0EzuaM890rW01z2oHW5zypsyuxdpmctII2q89AMdw="),
        (5098340, "af6bf5a05e042eb5ab2e", "UDkfGNpLwBlRkjdtfCQg8yNabaWgZCdI869s6p2Syxk="),
        (5098347, "ca1de12a03a72910791f", "37bAr4gxWS/C/Tr2LeeCwEX/9I/TfvaEYGbqVYxg8b8="),
    ];

    let blob_ids: Vec<BlobId> = test_cases
        .into_iter()
        .map(|(height, namespace, commitment)| {
            BlobId::new(
                Height::from(height),
                Namespace::const_v0(hex::decode(namespace).unwrap().try_into().unwrap()),
                Commitment::new(BASE64_STANDARD.decode(commitment).unwrap().try_into().unwrap())
            )
        })
        .collect();

    let mut test_cases: Vec<String> = vec![];

    for blob_id in blob_ids {

        println!("Getting inclusion data");
        let inclusion_data = loop {
            println!("blob_id: {}", blob_id);
            let data = match da_client.get_inclusion_data(&format!("{}", blob_id)).await {
                Ok(data) => data,
                Err(e) => {
                    writeln!(error_log, "Failed to get inclusion data: {}", e)
                        .expect("Failed to write to error log");
                    break Err(e);
                }
            };
            if let Some(data) = data {
                break Ok(data);
            }
            println!("Inclusion data not ready yet, retrying...");
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        };

        match inclusion_data {
            Ok(data) => {
                println!("Inclusion data for blob_id {}: {}", blob_id,hex::encode(data.data.clone()));
                test_cases.push(hex::encode(data.data.clone()));
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }

    let json = serde_json::to_string_pretty(&test_cases).expect("Failed to serialize test cases");
    std::fs::write("test_cases.json", json).expect("Failed to write test cases to file");
    println!("Wrote test cases to test_cases.json");

}