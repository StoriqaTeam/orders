use tokio_postgres;
use stq_http::errors::ControllerError;

#[derive(Debug, Fail)]
pub enum RepoError {
    #[fail(display = "Not found")]
    NotFound,
    #[fail(display = "Connection: {}", reason)]
    Connection { reason: String },
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
