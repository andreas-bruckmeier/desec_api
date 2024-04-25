use crate::{Client, Error};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

pub struct DomainClient<'a> {
    pub(crate) client: &'a crate::Client,
}

impl<'a> Client {
    pub fn domain(&'a self) -> DomainClient<'a> {
        DomainClient { client: self }
    }
}

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

pub type DomainList = Vec<Domain>;

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

    pub async fn get_domains(&self) -> Result<DomainList, Error> {
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

    pub async fn delete_domain(&self, domain: &str) -> Result<String, Error> {
        match self
            .client
            .delete(format!("/domains/{domain}/").as_str())
            .await
        {
            Ok(response) if response.status() == StatusCode::NO_CONTENT => response
                .text()
                .await
                .map_err(|error| Error::InvalidAPIResponse(error.to_string())),
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }

    pub async fn get_owning_domain(&self, qname: &str) -> Result<Domain, Error> {
        match self
            .client
            .get(format!("/domains/?owns_qname={qname}").as_str())
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

    pub async fn get_zonefile(&self, domain: &str) -> Result<String, Error> {
        match self
            .client
            .get(format!("/domains/{domain}/zonefile/").as_str())
            .await
        {
            Ok(response) if response.status() == StatusCode::OK => response
                .text()
                .await
                .map_err(|error| Error::InvalidAPIResponse(error.to_string())),
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }
}
