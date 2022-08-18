use near_jsonrpc_client::errors::JsonRpcError;
use near_jsonrpc_primitives::types::query::RpcQueryError;
use near_jsonrpc_primitives::types::transactions::RpcTransactionError;

#[derive(Debug, strum::EnumIter)]
pub enum ErrorKind {
    InvalidInput(String),
    InternalError(String),
    RPCError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Error {
    /// Code is a network-specific error code. If desired, this code can be
    /// equivalent to an HTTP status code.
    pub code: u32,

    /// Message is a network-specific error message.
    pub message: String,

    /// An error is retriable if the same request may succeed if submitted
    /// again.
    pub retriable: bool,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let retriable = if self.retriable { " (retriable)" } else { "" };
        write!(f, "Error #{}{}: {}", self.code, retriable, self.message)
    }
}

impl Error {
    pub fn from_error_kind(err: ErrorKind) -> Self {
        match err {
            ErrorKind::InvalidInput(message) => Self {
                code: 400,
                message: format!("Invalid Input: {}", message),
                retriable: false,
            },
            ErrorKind::InternalError(message) => Self {
                code: 500,
                message: format!("Internal Error: {}", message),
                retriable: true,
            },
            ErrorKind::RPCError(message) => Self {
                code: 500,
                message: format!("RPC error: {}", message),
                retriable: true,
            },
        }
    }
}

impl<T> From<T> for Error
where
    T: Into<ErrorKind>,
{
    fn from(err: T) -> Self {
        Self::from_error_kind(err.into())
    }
}

impl actix_web::ResponseError for Error {
    fn error_response(&self) -> actix_web::HttpResponse {
        let data = actix_web::web::Json(self);
        actix_web::HttpResponse::InternalServerError().json(data)
    }
}

impl From<JsonRpcError<RpcQueryError>> for ErrorKind {
    fn from(error: JsonRpcError<RpcQueryError>) -> Self {
        Self::RPCError(format!("{:#?}", error))
    }
}

impl From<JsonRpcError<RpcTransactionError>> for ErrorKind {
    fn from(error: JsonRpcError<RpcTransactionError>) -> Self {
        Self::RPCError(format!("{:#?}", error))
    }
}

impl From<serde_json::Error> for ErrorKind {
    fn from(error: serde_json::Error) -> Self {
        Self::InternalError(format!("Serialization failure: {:#?}", error))
    }
}

impl From<near_primitives::account::id::ParseAccountError> for ErrorKind {
    fn from(error: near_primitives::account::id::ParseAccountError) -> Self {
        Self::InternalError(format!("Could not parse account: {:#?}", error))
    }
}

impl<'a> From<&'a str> for ErrorKind {
    fn from(error: &'a str) -> Self {
        Self::InternalError(error.to_string())
    }
}
