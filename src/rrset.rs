use crate::{Client, Error};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

pub struct RrsetClient<'a> {
    pub(crate) client: &'a crate::Client,
}

impl<'a> Client {
    pub fn rrset(&'a self) -> RrsetClient<'a> {
        RrsetClient { client: self }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ResourceRecordSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rrset_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touched: Option<String>,
}

pub type ResourceRecordSetList = Vec<ResourceRecordSet>;

// Helper to generate a rate limit error from the response
async fn throttling_error(response: reqwest::Response) -> Error {
    match response.headers().get("retry-after") {
        Some(header) => match header.to_str() {
            Ok(header) => {
                Error::RateLimited(
                    header.to_string(),
                    response.text().await.unwrap_or_default()
                )
            },
            Err(_) => {
                Error::ApiError(
                    response.status().into(),
                    "Request got throttled with invalid retry-after header".to_string()
                )
            }
        },
        None => {
            Error::ApiError(
                response.status().into(),
                "Request got throttled without retry-header".to_string()
            )
        }
    }
}

impl<'a> RrsetClient<'a> {
    pub async fn create_rrset(
        &self,
        domain: String,
        subname: String,
        rrset_type: String,
        records: Vec<String>,
        ttl: u64,
    ) -> Result<ResourceRecordSet, Error> {
        let rrset = ResourceRecordSet {
            domain: Some(domain.clone()),
            subname: Some(subname),
            rrset_type: Some(rrset_type),
            records: Some(records),
            ttl: Some(ttl),
            ..ResourceRecordSet::default()
        };
        match self
            .client
            .post(
                format!("/domains/{domain}/rrsets/").as_str(),
                serde_json::to_string(&rrset)
                    .map_err(|error| Error::Serialize(error.to_string()))?,
            )
            .await
        {
            Ok(response) if response.status() == StatusCode::CREATED => response
                .json()
                .await
                .map_err(|error| Error::InvalidAPIResponse(error.to_string())),
            Ok(response) if response.status() == StatusCode::BAD_REQUEST => Err(Error::ApiError(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }

    pub async fn get_rrsets(&self, domain: &str) -> Result<ResourceRecordSetList, Error> {
        match self
            .client
            .get(format!("/domains/{domain}/rrsets/").as_str())
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => response
                .json()
                .await
                .map_err(|error| Error::InvalidAPIResponse(error.to_string())),
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }

    pub async fn get_rrset(
        &self,
        domain: &str,
        subname: &str,
        rrset_type: &str,
    ) -> Result<ResourceRecordSet, Error> {
        // https://desec.readthedocs.io/en/latest/dns/rrsets.html#accessing-the-zone-apex
        let subname = if subname.is_empty() { "@" } else { subname };

        match self
            .client
            .get(format!("/domains/{domain}/rrsets/{subname}/{rrset_type}/").as_str())
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => response
                .json()
                .await
                .map_err(|error| Error::InvalidAPIResponse(error.to_string())),
            Ok(response) if response.status() == StatusCode::NOT_FOUND => Err(Error::NotFound),
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }

    pub async fn update_rrset(
        &self,
        domain: &str,
        subname: &str,
        rrset_type: &str,
        patch: &ResourceRecordSet,
    ) -> Result<Option<ResourceRecordSet>, Error> {
        // https://desec.readthedocs.io/en/latest/dns/rrsets.html#accessing-the-zone-apex
        let subname = if subname.is_empty() { "@" } else { subname };

        match self
            .client
            .patch(
                format!("/domains/{domain}/rrsets/{subname}/{rrset_type}/").as_str(),
                serde_json::to_string(patch)
                .map_err(|error| Error::Serialize(error.to_string()))?,
            )
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => response
                .json()
                .await
                .map_err(|error| Error::InvalidAPIResponse(error.to_string())),

            // An exception to this rule is when an empty array is provided as the records field,
            // in which case the RRset is deleted and the return code is 204 No Content (cf. Deleting an RRset).
            Ok(response) if response.status() == StatusCode::NO_CONTENT => Ok(None),

            // In case the operation cannot be performed with the given parameters,
            // the API returns 400 Bad Request. This can happen, for instance, when there is 
            // a conflicting RRset with the same name and type, when not all required fields 
            // were provided correctly (such as, when the type value was not provided in uppercase),
            // or when the record content is semantically invalid 
            // (e.g. when you provide an unknown record type, or an A value that is not an IPv4 address).
            Ok(response) if response.status() == StatusCode::BAD_REQUEST => Err(Error::ApiError(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            // Rate limit / API Request Throttling
            Ok(response) if response.status() == StatusCode::TOO_MANY_REQUESTS => {
                Err(throttling_error(response).await)
            },
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }

    pub async fn delete_rrset(
        &self,
        domain: &str,
        subname: &str,
        rrset_type: &str,
    ) -> Result<(), Error> {
        // https://desec.readthedocs.io/en/latest/dns/rrsets.html#accessing-the-zone-apex
        let subname = if subname.is_empty() { "@" } else { subname };

        match self
            .client
            .delete(format!("/domains/{domain}/rrsets/{subname}/{rrset_type}/").as_str())
            .await
        {
            // Upon success or if the RRset did not exist in the first place,
            // the response status code is 204 No Content.
            Ok(response) if response.status() == StatusCode::NO_CONTENT => Ok(()),
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }
}
