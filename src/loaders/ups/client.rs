use std::fmt;

use failure::Error as FailureError;
use futures::future::IntoFuture;
use futures::prelude::*;
use futures::Stream;
use hyper::header::{self, Headers};
use serde_json::value::Value;
use tokio_core::reactor::Handle;

use stq_http::client::{Client as HttpClient, ClientHandle as HttpClientHandle, Config as HttpConfig};

use super::model::*;

const REQUEST_OPTION: &str = "1";
const CUSTOMER_CONTEXT: &str = "Storiqa";
const DELIVERED_ACTIVITY_STATE: &str = "D";

#[derive(Clone)]
pub struct UpsClient {
    handle: HttpClientHandle,
    access_license_number: String,
    url: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DeliveryState {
    NotDelivered,
    Delivered,
}

impl UpsClient {
    pub fn new(handle: &Handle, access_license_number: String, url: String) -> UpsClient {
        let client = HttpClient::new(
            &HttpConfig {
                http_client_retries: 3,
                http_client_buffer_size: 3,
                timeout_duration_ms: 5000,
            },
            handle,
        );
        let client_handle = client.handle();
        handle.spawn(client.stream().for_each(|_| Ok(())));
        UpsClient {
            handle: client_handle,
            access_license_number,
            url,
        }
    }

    pub fn delivery_status(&self, track_id: String) -> impl Future<Item = DeliveryState, Error = FailureError> {
        let self_clone = self.clone();
        request_body(self.access_license_number.clone(), track_id.clone())
            .into_future()
            .and_then(move |body| {
                self_clone
                    .handle
                    .request::<UpsResponse>(::hyper::Method::Post, self_clone.url.clone(), Some(body), Some(ups_headers()))
                    .map_err(From::from)
            }).and_then(move |res| DeliveryState::try_from_response(track_id, res))
            .map_err(From::from)
    }
}

impl DeliveryState {
    fn try_from_response(track_id: String, response: UpsResponse) -> Result<Self, FailureError> {
        if let Some(fault) = response.Fault {
            return Err(format_err!("Error in {} - {}", track_id, fault));
        }
        match response
            .TrackResponse
            .and_then(|r| r.Shipment.Package.and_then(|package| package.Activity))
        {
            Some(Value::Array(ser_activities)) => {
                let activities: Vec<Activity> = ::serde_json::from_value(Value::Array(ser_activities))?;
                Ok(Self::from_activities(&activities))
            }
            Some(Value::Object(ser_single_activity)) => {
                let single_activity: Activity = ::serde_json::from_value(Value::Object(ser_single_activity))?;
                Ok(Self::from_activities(&[single_activity]))
            }
            _ => Ok(DeliveryState::NotDelivered),
        }
    }

    fn from_activities(activities: &[Activity]) -> DeliveryState {
        if activities
            .iter()
            .filter_map(|activity| activity.Status.as_ref())
            .filter_map(|status| status.Type.as_ref())
            .any(|type_| type_ == DELIVERED_ACTIVITY_STATE)
        {
            DeliveryState::Delivered
        } else {
            DeliveryState::NotDelivered
        }
    }
}

fn request_body(access_license_number: String, track_id: String) -> Result<String, FailureError> {
    let request = UpsRequest {
        UPSSecurity: UPSSecurity {
            ServiceAccessToken: ServiceAccessToken {
                AccessLicenseNumber: access_license_number,
            },
        },
        TrackRequest: TrackRequest {
            Request: Request {
                RequestOption: REQUEST_OPTION.to_string(),
                TransactionReference: TransactionReference {
                    CustomerContext: CUSTOMER_CONTEXT.to_string(),
                },
            },
            InquiryNumber: track_id,
        },
    };
    let request_body = ::serde_json::to_string(&request)?;
    Ok(request_body)
}

fn ups_headers() -> Headers {
    let mut headers = Headers::new();
    headers.set(header::ContentType(::hyper::mime::APPLICATION_JSON));
    headers.set(header::AccessControlAllowOrigin::Any);
    headers.set(header::AccessControlAllowMethods(vec![::hyper::Method::Post]));
    headers
}

impl fmt::Display for DeliveryState {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            DeliveryState::NotDelivered => write!(f, "not delivered"),
            DeliveryState::Delivered => write!(f, "delivered"),
        }
    }
}
