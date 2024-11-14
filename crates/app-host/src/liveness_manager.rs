use jsonrpsee::http_client::HttpClient;
use tee::ProverRegistry;
use base::eth::{Eth, EthError};
use alloy::primitives::Address;
use tokio::time::Instant;
use std::time::{Duration, UNIX_EPOCH};

use rpc::ScrollSgxClient;

base::stack_error! {
    #[derive(Debug)]
    name: LivenessError,
    stack_name: LivenessErrorStack,
    error: {
        Eth(EthError),
        General(String),
        Fatal(String),
    },
    wrap: {
    },
    stack: {}
}

trait OkOrLivenessError<T> {
    fn ok_or_general_error(self) -> Result<T, LivenessError>;

    fn ok_or_fatal_error(self) -> Result<T, LivenessError>;
}

impl<T, E: core::fmt::Debug> OkOrLivenessError<T> for Result<T, E> {
    fn ok_or_general_error(self) -> Result<T, LivenessError> {
        self.map_err(|e| {
            LivenessError::General(format!("{e:?}"))
        })
    }

    fn ok_or_fatal_error(self) -> Result<T, LivenessError> {
        self.map_err(|e| {
            LivenessError::Fatal(format!("{e:?}"))
        })
    }
}

pub struct LivenessManager {
    enclave_client: HttpClient,
    prover_registry: ProverRegistry,

    address: Address,
    valid_until: u64,
    update_report_before_invalid_interval: u64,
}

impl LivenessManager {
    pub fn new(eth: Eth, liveness_contract: Address, enclave_client: HttpClient) -> Self {
        Self {
            prover_registry: ProverRegistry::new(eth, liveness_contract),
            enclave_client,
            address: Address::ZERO,
            valid_until: 0,
            update_report_before_invalid_interval: 600,
        }
    }

    fn get_next_report_instant(&self) -> Instant {
        let now_ts = std::time::SystemTime::now().duration_since(UNIX_EPOCH).expect("");
        
        let next_report_duration = Duration::from_secs(self.valid_until - self.update_report_before_invalid_interval);
        match next_report_duration.checked_sub(now_ts) {
            Some(gap) => Instant::now() + gap,
            None => Instant::now(),
        }
    }

    async fn register_new_report(&self) -> Result<tee::Registration, LivenessError> {
        let report = self.enclave_client.generate_attestation_report().await.ok_or_general_error()?;
        self.prover_registry.register(report).await.ok_or_general_error()
    }

    pub async fn start(&mut self) -> Result<(), LivenessError> {
        let address = self.enclave_client.get_address().await.ok_or_fatal_error()?;
        self.address = address;
        let registry = self.prover_registry.check_register_status(address).await.ok_or_fatal_error()?;
        if registry.address == address && registry.valid_until > 0 {
            log::info!("address {} already registered, valid_until {}", address, registry.valid_until);
            self.valid_until = registry.valid_until;
        }

        loop {
            if self.valid_until > 0 {
                tokio::time::sleep_until(self.get_next_report_instant()).await;
            }
            
            for i in 0..3 {
                match self.register_new_report().await {
                    Ok(registration) => {
                        self.address = registration.address;
                        self.valid_until = registration.valid_until;
                        break;
                    },
                    Err(err) => {
                        log::info!("failed to register new report, reason: {:?}, round: {}", err, i);
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                }
            }
            break Err(LivenessError::Fatal("failed to register report".to_string()))
        }
    }
}
