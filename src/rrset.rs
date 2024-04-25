use crate::{Client, Error};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

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
    pub created: String,
    pub domain: String,
    pub subname: String,
    pub name: String,
    #[serde(rename = "type")]
    pub rrset_type: String,
    pub records: Vec<String>,
    pub ttl: u64,
    pub touched: String,
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
        let rrset = json!({
            "subname": subname,
            "type": rrset_type,
            "ttl": ttl,
            "records": records
        });
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

    pub async fn patch_rrset_from(&self, rrset: &ResourceRecordSet) -> Result<Option<ResourceRecordSet>, Error> {
        self.patch_rrset(
            &rrset.domain,
            &rrset.subname,
            &rrset.rrset_type,
            &rrset.records,
            rrset.ttl
        ).await
    }

    pub async fn patch_rrset(
        &self,
        domain: &str,
        subname: &str,
        rrset_type: &str,
        records: &Vec<String>,
        ttl: u64,
    ) -> Result<Option<ResourceRecordSet>, Error> {
        // https://desec.readthedocs.io/en/latest/dns/rrsets.html#accessing-the-zone-apex
        let subname = if subname.is_empty() { "@" } else { subname };

        match self
            .client
            .patch(
                format!("/domains/{domain}/rrsets/{subname}/{rrset_type}/").as_str(),
                serde_json::to_string(
                    &json!({
                        "ttl": ttl,
                        "records": records
                    })
                )
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
