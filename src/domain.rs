use crate::{Client, Error};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

/// An asynchronous client to work with the deSEC domain API.
pub struct DomainClient<'a> {
    pub(crate) client: &'a crate::Client,
}

impl<'a> Client {
    /// Returns a wrapping client for the domain API.
    pub fn domain(&'a self) -> DomainClient<'a> {
        DomainClient { client: self }
    }
}

/// Representation of a deSEC [`domain`][reference].
///
/// [reference]: https://desec.readthedocs.io/en/latest/dns/domains.html#domain-field-reference
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Domain {
    pub created: String,
    pub keys: Vec<DNSSECKeyInfo>,
    pub minimum_ttl: u16,
    pub name: String,
    pub published: String,
    pub touched: String,
    pub zonefile: String,
}

/// Representation of a deSEC [`DNSSEC`][reference] key.
///
/// [reference]: https://desec.readthedocs.io/en/latest/dns/domains.html#domain-field-reference
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DNSSECKeyInfo {
    pub dnskey: String,
    pub ds: Vec<String>,
    #[serde(rename = "flags")]
    pub keyflags: u16,
    pub keytype: String,
    pub managed: bool,
}

impl<'a> DomainClient<'a> {
    /// Creates a new domain and returns the newly created [`Domain`][domain].
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::ApiError`][error] This can happen when the request payload was malformed, or when the requested
    ///   domain name is unavailable (because it conflicts with another user’s zone) or invalid (due to policy, see below).
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    /// [domain]: ./struct.Domain.html
    pub async fn create_domain(&self, domain: String) -> Result<Domain, Error> {
        match self
            .client
            .post("/domains/", format!("{{\"name\": \"{domain}\"}}"))
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => response
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

    /// Retrieves a list of all domains that you own in the account.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into a vector of [`desec_api::domain::Domain`][domain] objects
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    /// [domain]: ./struct.Domain.html
    pub async fn get_domains(&self) -> Result<Vec<Domain>, Error> {
        match self.client.get("/domains/").await {
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

    /// Retrieves a specific domain of your account.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::NotFound`][error] if the RRSet does not exist or does not belong to you
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into a [`desec_api::domain::Domain`][domain] object
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    /// [domain]: ./struct.Domain.html
    pub async fn get_domain(&self, domain: &str) -> Result<Domain, Error> {
        match self
            .client
            .get(format!("/domains/{domain}/").as_str())
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

    /// Deletes the given domain from your account.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    /// [domain]: ./struct.Domain.html
    pub async fn delete_domain(&self, domain: &str) -> Result<(), Error> {
        match self
            .client
            .delete(format!("/domains/{domain}/").as_str())
            .await
        {
            Ok(response) if response.status() == StatusCode::NO_CONTENT => Ok(()),
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }

    /// Returns the account-domain which is responsible for the given DNS name.
    ///
    /// Let’s say you have the domains example.net, dev.example.net and git.dev.example.net,
    /// and you would like to request a certificate for the TLS server name www.dev.example.net.
    /// In this case, the TXT record needs to be created with the name _acme-challenge.www.dev.example.net.
    ///
    /// If your account has a domain that is responsible for the name qname, the API returns a JSON array
    /// containing only that domain object in the response body. Otherwise, the JSON array will be empty.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into a vector of [`desec_api::domain::Domain`][domain] objects
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    /// [domain]: ./struct.Domain.html
    pub async fn get_owning_domain(&self, qname: &str) -> Result<Vec<Domain>, Error> {
        match self
            .client
            .get(format!("/domains/?owns_qname={qname}").as_str())
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

    /// Returns the zone file for the given domain in plain text format.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    /// [domain]: ./struct.Domain.html
    pub async fn get_zonefile(&self, domain: &str) -> Result<String, Error> {
        match self
            .client
            .get(format!("/domains/{domain}/zonefile/").as_str())
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => {
                response.text().await.map_err(Error::Reqwest)
            }
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }
}
