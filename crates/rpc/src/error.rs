use core::fmt::Debug;

use jsonrpsee::types::{
    error::{INTERNAL_ERROR_CODE, INTERNAL_ERROR_MSG, INVALID_PARAMS_CODE, INVALID_PARAMS_MSG},
    ErrorObjectOwned,
};

pub trait OkOrInternalError<T> {
    fn ok_or_internal_error(self) -> Result<T, ErrorObjectOwned>;
}

impl<T, E: Debug> OkOrInternalError<T> for Result<T, E> {
    fn ok_or_internal_error(self) -> Result<T, ErrorObjectOwned> {
        self.map_err(|e| {
            ErrorObjectOwned::owned(
                INTERNAL_ERROR_CODE,
                INTERNAL_ERROR_MSG,
                Some(format!("{e:?}")),
            )
        })
    }
}

pub trait OkOrInvalidParams<T> {
    fn ok_or_invalid_params(self) -> Result<T, ErrorObjectOwned>;
}

impl<T, E: Debug> OkOrInvalidParams<T> for Result<T, E> {
    fn ok_or_invalid_params(self) -> Result<T, ErrorObjectOwned> {
        self.map_err(|e| {
            ErrorObjectOwned::owned(
                INVALID_PARAMS_CODE,
                INVALID_PARAMS_MSG,
                Some(format!("{e:?}")),
            )
        })
    }
}

pub fn invalid_params(e: impl ToString) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(INVALID_PARAMS_CODE, INVALID_PARAMS_MSG, Some(e.to_string()))
}
