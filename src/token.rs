use crate::{Client, Error};
use core::convert::From;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// An asynchronous client to work with the deSEC token API.
pub struct TokenClient<'a> {
    pub(crate) client: &'a crate::Client,
}

impl<'a> Client {
    /// Returns a wrapping client for the token API.
    pub fn token(&'a self) -> TokenClient<'a> {
        TokenClient { client: self }
    }
}

/// Representation of a deSEC [`token`][reference].
///
/// [reference]: https://desec.readthedocs.io/en/latest/auth/tokens.html#token-field-reference
#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
    pub created: String,
    pub id: String,
    pub last_used: Option<String>,
    pub name: String,
    pub perm_manage_tokens: bool,
    pub allowed_subnets: Vec<String>,
    pub max_age: Option<String>,
    pub max_unused_period: Option<String>,
    pub token: Option<String>,
}

impl<'a> TokenClient<'a> {
    /// Creates a new token.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::token::Token
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn create_token(
        &self,
        name: Option<String>,
        allowed_subnets: Option<Vec<String>>,
        perm_manage_tokens: Option<bool>,
        max_age: Option<String>,
        max_unused_period: Option<String>,
    ) -> Result<Token, Error> {
        // Construct payload
        let mut payload_map = Map::new();
        if let Some(name) = name {
            payload_map.insert("name".to_string(), Value::String(name));
        }
        if let Some(allowed_subnets) = allowed_subnets {
            payload_map.insert("allowed_subnets".to_string(), Value::from(allowed_subnets));
        }
        if let Some(perm_manage_tokens) = perm_manage_tokens {
            payload_map.insert(
                "perm_manage_tokens".to_string(),
                Value::Bool(perm_manage_tokens),
            );
        }
        if let Some(max_age) = max_age {
            payload_map.insert("max_age".to_string(), Value::String(max_age));
        }
        if let Some(max_unused_period) = max_unused_period {
            payload_map.insert(
                "max_unused_period".to_string(),
                Value::String(max_unused_period),
            );
        }
        let payload = Some(serde_json::to_string(&payload_map).unwrap());
        // Send create token request
        let response = self.client.post("/auth/tokens/", payload).await?;
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

    /// Deletes a token.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn delete_token(&self, token_id: &str) -> Result<(), Error> {
        let response = self
            .client
            .delete(format!("/auth/tokens/{token_id}/").as_str())
            .await?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            _ => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
        }
    }

    /// List all tokens.
    ///
    /// Up to 500 items are returned at a time. Pagination is currently no implemented by this client.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into a list of desec_api::token::Token
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn list(&self) -> Result<Vec<Token>, Error> {
        let response = self.client.get("/auth/tokens/").await?;
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

    /// Retrieves a specific token.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::token::Toke
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn get(&self, token_id: &str) -> Result<Token, Error> {
        let response = self
            .client
            .get(format!("/auth/tokens/{token_id}/").as_str())
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

    /// Update token.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::token::Toke
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn patch_token(
        &self,
        token_id: &str,
        name: Option<String>,
        allowed_subnets: Option<Vec<String>>,
        perm_manage_tokens: Option<bool>,
        max_age: Option<String>,
        max_unused_period: Option<String>,
    ) -> Result<Token, Error> {
        // Construct payload
        let mut payload_map = Map::new();
        if let Some(name) = name {
            payload_map.insert("name".to_string(), Value::String(name));
        }
        if let Some(allowed_subnets) = allowed_subnets {
            payload_map.insert("allowed_subnets".to_string(), Value::from(allowed_subnets));
        }
        if let Some(perm_manage_tokens) = perm_manage_tokens {
            payload_map.insert(
                "perm_manage_tokens".to_string(),
                Value::Bool(perm_manage_tokens),
            );
        }
        if let Some(max_age) = max_age {
            payload_map.insert("max_age".to_string(), Value::String(max_age));
        }
        if let Some(max_unused_period) = max_unused_period {
            payload_map.insert(
                "max_unused_period".to_string(),
                Value::String(max_unused_period),
            );
        }
        let payload = serde_json::to_string(&payload_map).unwrap();
        let response = self
            .client
            .patch(format!("/auth/tokens/{token_id}/").as_str(), payload)
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
}
