use base::eth::EthError;

type CurrentBatchIndex = u64;
type NextBatchIndexForBundle = u64;

base::stack_error! {
    #[derive(Debug)]
    name: StateManagerError,
    stack_name: StateManagerErrorStack,
    error: {
        Eth(EthError),
        General(String),
        Fatal(String),
        BatchesNotEnough(CurrentBatchIndex, NextBatchIndexForBundle),
    },
    wrap: {
    },
    stack: {}
}

impl From<EthError> for StateManagerError {
    fn from(value: EthError) -> Self {
        Self::Eth(value)
    }
}

pub trait OkOrStateManagerError<T> {
    fn ok_or_general_error(self) -> Result<T, StateManagerError>;

    fn ok_or_fatal_error(self) -> Result<T, StateManagerError>;
}

impl<T, E: core::fmt::Debug> OkOrStateManagerError<T> for Result<T, E> {
    fn ok_or_general_error(self) -> Result<T, StateManagerError> {
        self.map_err(|e| StateManagerError::General(format!("{e:?}")))
    }

    fn ok_or_fatal_error(self) -> Result<T, StateManagerError> {
        self.map_err(|e| StateManagerError::Fatal(format!("{e:?}")))
    }
}
