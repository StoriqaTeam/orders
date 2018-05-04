use failure::Error;
use stq_http::errors::ControllerError;
use tokio_postgres;

#[derive(Debug, Fail)]
pub enum RepoError {
    #[fail(display = "Not found")]
    NotFound,
    #[fail(display = "Connection: {}", reason)]
    Connection { reason: String },
    #[fail(display = "Other: {}, statement: {:?}", error, statement)]
    Other {
        error: Error,
        statement: Option<String>,
    },
}

impl From<tokio_postgres::Error> for RepoError {
    fn from(v: tokio_postgres::Error) -> Self {
        RepoError::Connection {
            reason: format!("{}", v),
        }
    }
}

impl From<RepoError> for ControllerError {
    fn from(e: RepoError) -> Self {
        ControllerError::InternalServerError(e.into())
    }
}

#[derive(Debug, Fail)]
pub enum AuthorizationError {
    #[fail(display = "Missing user_id")]
    Missing,
    #[fail(display = "Failed to parse user_id: {}, {}", raw, error)]
    Parse { raw: String, error: Error },
}
