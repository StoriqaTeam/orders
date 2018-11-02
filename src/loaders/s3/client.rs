use std::sync::Arc;

use failure::Error as FailureError;
use futures::future;
use futures::future::{err, ok, FutureResult};
use futures::prelude::*;
use rusoto_core::credential::{AwsCredentials, CredentialsError};
use rusoto_core::request::HttpClient;
use rusoto_core::ProvideAwsCredentials;
use rusoto_core::Region;
use rusoto_s3::PutObjectError;
use rusoto_s3::{PutObjectRequest, S3Client as RusotoS3Client, StreamingBody, S3};

use config;

#[derive(Clone)]
pub struct S3Client {
    s3_upload: Arc<S3Upload>,
}

#[derive(Debug, Fail)]
enum S3Error {
    #[fail(display = "Access Error: {}", _0)]
    Access(String),
    #[fail(display = "Network Error: {}", _0)]
    Network(String),
    #[fail(display = "Validation error: {}", _0)]
    Validation(String),
    #[fail(display = "Unknown error: {}", _0)]
    Unknown(String),
}

struct RusotoS3Upload {
    rusoto_s3: RusotoS3Client,
    acl: String,
    bucket: String,
    region: Region,
}

struct DummyS3Upload;

trait S3Upload {
    fn upload(&self, name: &str, data: Vec<u8>) -> Box<Future<Item = (), Error = FailureError>>;
}

impl S3Client {
    pub fn new(config: config::S3) -> Result<Self, FailureError> {
        let client = HttpClient::new()?;
        let region = config.region.parse::<Region>()?;
        let rusoto_s3_clinet = RusotoS3Client::new_with(client, config.clone(), region.clone());
        Ok(S3Client {
            s3_upload: Arc::new(RusotoS3Upload {
                rusoto_s3: rusoto_s3_clinet,
                bucket: config.bucket,
                acl: config.acl,
                region,
            }),
        })
    }

    pub fn create_dummy() -> Self {
        S3Client {
            s3_upload: Arc::new(DummyS3Upload),
        }
    }

    pub fn upload(&self, name: &str, data: Vec<u8>) -> impl Future<Item = (), Error = FailureError> {
        self.s3_upload.upload(name, data)
    }
}

impl S3Upload for RusotoS3Upload {
    fn upload(&self, name: &str, data: Vec<u8>) -> Box<Future<Item = (), Error = FailureError>> {
        info!(
            "uploading {} bytes to https://s3.{}.amazonaws.com/{}/{}",
            data.len(),
            self.region.name(),
            self.bucket,
            name
        );
        let request = PutObjectRequest {
            acl: Some(self.acl.clone()),
            body: Some(StreamingBody::from(data)),
            bucket: self.bucket.clone(),
            key: name.to_string(),
            content_type: Some("text/csv".to_string()),
            ..Default::default()
        };

        let res = self
            .rusoto_s3
            .put_object(request)
            .map(|_| ())
            .map_err(S3Error::from)
            .map_err(From::from);
        Box::new(res)
    }
}

impl S3Upload for DummyS3Upload {
    fn upload(&self, _name: &str, _data: Vec<u8>) -> Box<Future<Item = (), Error = FailureError>> {
        Box::new(future::err(format_err!("S3 client is not properly configured")))
    }
}

impl ProvideAwsCredentials for config::S3 {
    type Future = FutureResult<AwsCredentials, CredentialsError>;

    fn credentials(&self) -> Self::Future {
        ok(AwsCredentials::new(self.key.clone(), self.secret.clone(), None, None))
    }
}

impl<T: 'static> Into<Box<Future<Item = T, Error = S3Error>>> for S3Error {
    fn into(self) -> Box<Future<Item = T, Error = S3Error>> {
        Box::new(err::<T, _>(self))
    }
}

impl From<PutObjectError> for S3Error {
    fn from(e: PutObjectError) -> Self {
        match e {
            PutObjectError::HttpDispatch(err) => S3Error::Network(format!("{}", err)),
            PutObjectError::Credentials(err) => S3Error::Access(format!("{}", err)),
            PutObjectError::Validation(err) => S3Error::Validation(format!("{}", err)),
            PutObjectError::Unknown(err) => S3Error::Unknown(format!("{}", err)),
        }
    }
}
