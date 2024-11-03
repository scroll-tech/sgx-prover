use base::eth::EthError;
use anyhow::Error as AnyError;


pub fn convert_eth_error(eth_error: EthError) -> AnyError {
    if let Some(data) = eth_error.revert() {
        anyhow::anyhow!(&data)
    } else {
        anyhow::anyhow!("unknown eth error")
    }
}