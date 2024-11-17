use alloy::primitives::Address;

use jsonrpsee::http_client::HttpClient;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tee::ProverRegistry;
use tokio::time::Instant;

use rpc::ScrollSgxClient;

base::stack_error! {
    #[derive(Debug)]
    name: LivenessError,
    stack_name: LivenessErrorStack,
    error: {
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
        self.map_err(|e| LivenessError::General(format!("{e:?}")))
    }

    fn ok_or_fatal_error(self) -> Result<T, LivenessError> {
        self.map_err(|e| LivenessError::Fatal(format!("{e:?}")))
    }
}

pub struct AddressInfo {
    pub address: Address,
    pub valid_until: u64,
}

impl AddressInfo {
    fn init() -> Self {
        Self {
            address: Address::ZERO,
            valid_until: 0,
        }
    }

    fn empty(&self) -> bool {
        return self.valid_until == 0;
    }
}

pub struct LivenessManager {
    enclave_client: HttpClient,
    prover_registry: Arc<ProverRegistry>,

    address_info: Arc<Mutex<AddressInfo>>,
    update_report_before_invalid_interval: u64,
}

impl LivenessManager {
    pub async fn new(
        prover_registry: Arc<ProverRegistry>,
        enclave_client: HttpClient,
    ) -> Result<Self, LivenessError> {
        let manager = Self {
            prover_registry,
            enclave_client,
            address_info: Arc::new(Mutex::new(AddressInfo::init())),
            update_report_before_invalid_interval: 600,
        };

        manager.register_and_init_address_info().await?;
        Ok(manager)
    }

    async fn register_and_init_address_info(&self) -> Result<(), LivenessError> {
        let address = self
            .enclave_client
            .get_address()
            .await
            .ok_or_fatal_error()?;

        let registry = self
            .prover_registry
            .check_register_status(address)
            .await
            .ok_or_fatal_error()?;
        if registry.address == address && registry.valid_until > 0 {
            log::info!(
                "address {} already registered, valid_until {}",
                address,
                registry.valid_until
            );
            self.set_address_info(address, registry.valid_until);
        } else {
            let registration = self.register_new_report().await?;
            log::info!(
                "address {} already registered, valid_until {}",
                address,
                registry.valid_until
            );
            self.set_address_info(registration.address, registration.valid_until);
        }
        Ok(())
    }

    fn get_next_report_instant(&self, valid_until: u64) -> Instant {
        let now_ts = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();

        let next_report_duration =
            Duration::from_secs(valid_until - self.update_report_before_invalid_interval);
        match next_report_duration.checked_sub(now_ts) {
            Some(gap) => Instant::now() + gap,
            None => Instant::now(),
        }
    }

    async fn register_new_report(&self) -> Result<tee::Registration, LivenessError> {
        let report = self
            .enclave_client
            .generate_attestation_report()
            .await
            .ok_or_general_error()?;
        self.prover_registry
            .register(report)
            .await
            .ok_or_general_error()
    }

    fn set_address_info(&self, address: Address, valid_until: u64) {
        let mut info = self.address_info.lock().unwrap();
        info.address = address;
        info.valid_until = valid_until;
    }

    fn get_address_info(&self) -> AddressInfo {
        let info = self.address_info.lock().unwrap();
        AddressInfo {
            address: info.address,
            valid_until: info.valid_until,
        }
    }

    pub fn get_shared_address_info(&self) -> Arc<Mutex<AddressInfo>> {
        self.address_info.clone()
    }

    pub async fn start(&self) -> Result<(), LivenessError> {
        loop {
            let address_info = self.get_address_info();
            if !address_info.empty() {
                tokio::time::sleep_until(self.get_next_report_instant(address_info.valid_until))
                    .await;
            }

            for i in 0..3 {
                match self.register_new_report().await {
                    Ok(registration) => {
                        self.set_address_info(registration.address, registration.valid_until);
                        break;
                    }
                    Err(err) => {
                        log::info!(
                            "failed to register new report, reason: {:?}, round: {}",
                            err,
                            i
                        );
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                }
            }
            break Err(LivenessError::Fatal(
                "failed to register report".to_string(),
            ));
        }
    }
}
