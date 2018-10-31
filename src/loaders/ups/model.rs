use std::fmt;

use serde_json::value::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct UpsResponse {
    pub Fault: Option<Fault>,
    pub TrackResponse: Option<TrackResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TrackResponse {
    pub Response: Response,
    pub Shipment: Shipment,
    pub Disclaimer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Response {
    pub ResponseStatus: ResponseStatus,
    pub TransactionReference: TransactionReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ResponseStatus {
    pub Code: String,
    pub Description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Shipment {
    pub Package: Option<Package>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Package {
    pub Activity: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Activity {
    pub Status: Option<Status>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Status {
    pub Type: Option<String>,
    pub Description: String,
    pub Code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct UpsRequest {
    pub UPSSecurity: UPSSecurity,
    pub TrackRequest: TrackRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TrackRequest {
    pub Request: Request,
    pub InquiryNumber: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Request {
    pub RequestOption: String,
    pub TransactionReference: TransactionReference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TransactionReference {
    pub CustomerContext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct UPSSecurity {
    pub ServiceAccessToken: ServiceAccessToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ServiceAccessToken {
    pub AccessLicenseNumber: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fault {
    pub faultcode: String,
    pub faultstring: String,
    pub detail: FaultDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct FaultDetail {
    pub Errors: Errors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Errors {
    pub ErrorDetail: ErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ErrorDetail {
    pub Severity: String,
    pub PrimaryErrorCode: ErrorCode,
    pub Location: Option<Location>,
    pub SubErrorCode: Option<ErrorCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ErrorCode {
    pub Code: String,
    pub Description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Location {
    pub LocationElementName: String,
    pub XPathOfElement: String,
    pub OriginalValue: String,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let rendered = ::serde_json::to_string(&self).map_err(|_| fmt::Error)?;
        write!(f, "{}", rendered)
    }
}
