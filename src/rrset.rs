use crate::{Client, Error};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// An asynchronous client to create, update or delete so-called Resource Record Sets (RRsets).
pub struct RrsetClient<'a> {
    pub(crate) client: &'a crate::Client,
}

impl<'a> Client {
    /// Returns a wrapping client for the Resource Record Sets (RRsets) API.
    pub fn rrset(&'a self) -> RrsetClient<'a> {
        RrsetClient { client: self }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ResourceRecordSet {
    pub created: String,
    pub domain: String,
    /// Subname is optional, so you can select the [zone apex][link]
    ///
    /// [link]: https://desec.readthedocs.io/en/latest/dns/rrsets.html#accessing-the-zone-apex
    pub subname: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub rrset_type: String,
    pub ttl: u64,
    pub records: Vec<String>,
    pub touched: String,
}

impl<'a> RrsetClient<'a> {
    /// Creates a new RRSet and returns the newly created [`ResourceRecordSet`][rrset].
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::ApiError`][error] In case the operation cannot be performed with the given parameters.
    ///   This can happen, for instance, when there is a conflicting RRset with the same name and type,
    ///   when not all required fields were provided correctly (such as, when the type value was not provided in uppercase),
    ///   or when the record content is semantically invalid (e.g. when you provide an unknown record type,
    ///   or an A value that is not an IPv4 address).
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    /// [rrset]: ./struct.ResourceRecordSet.html
    pub async fn create_rrset(
        &self,
        domain: &str,
        subname: Option<&str>,
        rrset_type: &str,
        ttl: u64,
        records: &Vec<String>,
    ) -> Result<ResourceRecordSet, Error> {
        let rrset = json!({
            "subname": subname.unwrap_or("@"),
            "type": rrset_type,
            "ttl": ttl,
            "records": records
        });
        let response = self
            .client
            .post(
                format!("/domains/{domain}/rrsets/").as_str(),
                Some(
                    serde_json::to_string(&rrset)
                        .map_err(|error| Error::Serialize(error.to_string()))?,
                ),
            )
            .await?;
        match response.status() {
            StatusCode::CREATED => {
                let response_text = response.text().await.map_err(Error::Reqwest)?;
                serde_json::from_str(&response_text)
                    .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))
            }
            _ => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
        }
    }

    /// Retrieves a list of all RRSets that you own in the given domain.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into a vector of desec_api::rrset::ResourceRecordSet objects
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn get_rrsets(&self, domain: &str) -> Result<Vec<ResourceRecordSet>, Error> {
        let response = self
            .client
            .get(format!("/domains/{domain}/rrsets/").as_str())
            .await?;
        match response.status() {
            StatusCode::OK => {
                let response_text = response.text().await.map_err(Error::Reqwest)?;
                serde_json::from_str(&response_text)
                    .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))
            }
            _ => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
        }
    }

    /// Retrieves a specific RRSet.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::NotFound`][error] if the RRSet does not exist or does not belong to you
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn get_rrset(
        &self,
        domain: &str,
        subname: Option<&str>,
        rrset_type: &str,
    ) -> Result<ResourceRecordSet, Error> {
        // https://desec.readthedocs.io/en/latest/dns/rrsets.html#accessing-the-zone-apex
        let subname = subname.unwrap_or("@");
        let response = self
            .client
            .get(format!("/domains/{domain}/rrsets/{subname}/{rrset_type}/").as_str())
            .await?;
        match response.status() {
            StatusCode::OK => {
                let response_text = response.text().await.map_err(Error::Reqwest)?;
                serde_json::from_str(&response_text)
                    .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))
            }
            _ => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
        }
    }

    /// Updates an existing RRSet based on the given RRSet.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::Serialize`][error] if the given RRSet cannot be serialized (is this even possible?)
    /// - [`Error::RateLimited`][error] if you hit a rate limit by making to many requests
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn patch_rrset_from(
        &self,
        rrset: &ResourceRecordSet,
    ) -> Result<Option<ResourceRecordSet>, Error> {
        self.patch_rrset(
            &rrset.domain,
            rrset.subname.as_deref(),
            &rrset.rrset_type,
            &rrset.records,
            rrset.ttl,
        )
        .await
    }

    /// Updates an existing RRSet based on the given values.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::Serialize`][error] if the given RRSet cannot be serialized (is this even possible?)
    /// - [`Error::RateLimited`][error] if you hit a rate limit by making to many requests
    /// - [`Error::ApiError`][error] In case the operation cannot be performed with the given parameters.
    ///   This can happen, for instance, when there is a conflicting RRset with the same name and type,
    ///   when not all required fields were provided correctly (such as, when the type value was not provided in uppercase),
    ///   or when the record content is semantically invalid (e.g. when you provide an unknown record type,
    ///   or an A value that is not an IPv4 address).
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn patch_rrset(
        &self,
        domain: &str,
        subname: Option<&str>,
        rrset_type: &str,
        records: &[String],
        ttl: u64,
    ) -> Result<Option<ResourceRecordSet>, Error> {
        // https://desec.readthedocs.io/en/latest/dns/rrsets.html#accessing-the-zone-apex
        let subname = subname.unwrap_or("@");

        let response = self
            .client
            .patch(
                format!("/domains/{domain}/rrsets/{subname}/{rrset_type}/").as_str(),
                serde_json::to_string(&json!({
                    "ttl": ttl,
                    "records": records
                }))
                .map_err(|error| Error::Serialize(error.to_string()))?,
            )
            .await?;
        match response.status() {
            StatusCode::OK => {
                let response_text = response.text().await.map_err(Error::Reqwest)?;
                serde_json::from_str(&response_text)
                    .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))
            }
            StatusCode::NO_CONTENT => Ok(None),
            _ => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
        }
    }

    /// Deletes the RRSet specified by the given domain, subname and type.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn delete_rrset(
        &self,
        domain: &str,
        subname: Option<&str>,
        rrset_type: &str,
    ) -> Result<(), Error> {
        // https://desec.readthedocs.io/en/latest/dns/rrsets.html#accessing-the-zone-apex
        let subname = subname.unwrap_or("@");
        let response = self
            .client
            .delete(format!("/domains/{domain}/rrsets/{subname}/{rrset_type}/").as_str())
            .await?;
        match response.status() {
            // Upon success or if the RRset did not exist in the first place,
            // the response status code is 204 No Content.
            StatusCode::NO_CONTENT => Ok(()),
            _ => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
        }
    }
}
