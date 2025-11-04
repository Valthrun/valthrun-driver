use thiserror::Error;

pub type MetricsResult<T> = Result<T, MetricsError>;

#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("reading public key: {0}")]
    CryptoPublicKeyImport(rsa::pkcs8::spki::Error),

    #[error("encrypt: {0}")]
    CryptoEncryptPayload(aes_gcm::Error),

    #[error("encrypt header: {0}")]
    CryptoEncryptHeader(rsa::Error),

    #[error("invalid port")]
    InvalidPort,

    #[error("encode: {0}")]
    EncodeFailure(serde_json::Error),

    #[error("decode: {0}")]
    DecodeFailure(serde_json::Error),

    #[error("{0}")]
    HttpError(ureq::Error),

    #[error("http status code {0}")]
    HttpStatusCodeIndicatesFailure(u16),

    #[error("rate limited")]
    ResponseRateLimited,

    #[error("generic server error")]
    ResponseGenericServerError,

    #[error("unknown")]
    Unknown,
}
