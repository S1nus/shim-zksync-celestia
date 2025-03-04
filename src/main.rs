use zksync_da_clients::celestia::CelestiaClient;
use zksync_config::{configs::da_client::CelestiaConfig, CelestiaConfig};

fn main() {
    let config = CelestiaConfig {
        api_node_url: "https://rpc.celestia.org".to_string(),
        eq_service_url: "https://eq-service.celestia.org".to_string(),
        namespace: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        chain_id: "2222-2".to_string(),
        timeout_ms: 10000,
        tm_rpc_url: "https://rpc.celestia.org".to_string(),
    };
    
}