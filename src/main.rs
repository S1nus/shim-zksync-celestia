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
//use eq_sdk::BlobId;
//use celestia_types::{blob::Co}
//use celestia_types::{blob::Commitment, block::Height as BlockHeight, nmt::Namespace};

#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = CelestiaConfig {
        api_node_url: "https://grpc.archive.mocha.cumulo.com.es:443".to_string(),
        //eq_service_url: "https://eq-service-dev.eu-north-2.gateway.fm".to_string(),
        eq_service_url: "http://eqs.cnode.phd:50051".to_string(),
        namespace: "00000000000000000000000000000000000000000413528b469e1926".to_string(),
        //ychain_id: "2222-2".to_string(),
        chain_id: "mocha-4".to_string(),
        timeout_ms: 10000,
        tm_rpc_url: "http://public-celestia-mocha4-consensus.numia.xyz:26657".to_string(),
    };

    let secrets = CelestiaSecrets {
        private_key: env::var("PRIVATE_KEY")
            .expect("PRIVATE_KEY environment variable not set")
            .parse()
            .expect("Failed to parse PRIVATE_KEY"),
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

    let mut count = 0;
    loop {
        println!("Run number: {}", count);
        let mut random_data = vec![0u8; blob_size];
        rand::thread_rng().fill_bytes(&mut random_data);

        println!("Dispatching blob");
        let blob = match da_client.dispatch_blob(1, random_data).await {
            Ok(blob) => blob,
            Err(e) => {
                writeln!(error_log, "Failed to dispatch blob: {}", e)
                    .expect("Failed to write to error log");
                continue;
            }
        };
        println!("Blob dispatched: {:?}", blob.blob_id);

        println!("Getting inclusion data");
        let inclusion_data = loop {
            let data = match da_client.get_inclusion_data(&blob.blob_id).await {
                Ok(data) => data,
                Err(e) => {
                    writeln!(error_log, "Failed to get inclusion data: {}", e)
                        .expect("Failed to write to error log");
                    continue;
                }
            };
            if let Some(data) = data {
                break data;
            }
            println!("Inclusion data not ready yet, retrying...");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        };

        println!("Inclusion data: {:?}", inclusion_data.data);
        count += 1;
    }
}