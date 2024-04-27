use reqwest::{header, Response};
use thiserror::Error;

pub mod account;
pub mod domain;
pub mod rrset;

static API_URL: &str = "https://desec.io/api/v1";

#[derive(Error, Debug)]
pub enum Error {
    #[error("An error occurred during the request")]
    Reqwest(reqwest::Error),
    // Could be integer but not header also allows http-dates
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Retry-After
    #[error("You hit a rate limit and need to wait {0} seconds. Additional Info: {1}")]
    RateLimited(String, String),
    #[error("The requested resource does not exist or you are not the owner")]
    NotFound,
    #[error("API returned status code {0} with message '{1}'")]
    ApiError(u16, String),
    #[error("API returned undocumented status code {0} with message '{1}'")]
    UnexpectedStatusCode(u16, String),
    #[error("API returned an invalid response. error: {0}, body: {1}")]
    InvalidAPIResponse(String, String),
    #[error("An error occurred while serializing a JSON value: {0}")]
    Serialize(String),
    #[error("Failed to create HTTP client: {0}")]
    ReqwestClientBuilder(String),
}

#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    pub api_url: String,
    pub token: String,
}

impl Client {
    /// Creates a new client using the given API token.
    ///
    /// # Errors
    ///
    /// This method fails with [`Error::ReqwestClientBuilder`][error] if the underlying [`reqwest::ClientBuilder`][builder] fails to build a http client.
    ///
    /// [error]: enum.Error.html
    /// [builder]: https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html#method.build
    pub fn new(token: String) -> Result<Self, Error> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(format!("Token {}", token.as_str()).as_str()).unwrap(),
        );
        let client = reqwest::ClientBuilder::new()
            .user_agent("rust-desec-client")
            .default_headers(headers)
            .build()
            .map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
        Ok(Client {
            client,
            api_url: API_URL.into(),
            token,
        })
    }

    async fn get(&self, endpoint: &str) -> Result<Response, reqwest::Error> {
        self.client
            .get(format!("{}{}", self.api_url, endpoint))
            .send()
            .await
    }

    async fn post(&self, endpoint: &str, body: Option<String>) -> Result<Response, reqwest::Error> {
        // TODO replace if/else with something smarter
        if body.is_some() {
            self.client
                .post(format!("{}{}", self.api_url, endpoint).as_str())
                .header("Content-Type", "application/json")
                .body(body.unwrap())
                .send()
                .await
        } else {
            self.client
                .post(format!("{}{}", self.api_url, endpoint).as_str())
                .header("Content-Type", "application/json")
                .send()
                .await
        }
    }

    async fn patch(&self, endpoint: &str, body: String) -> Result<Response, reqwest::Error> {
        self.client
            .patch(format!("{}{}", self.api_url, endpoint).as_str())
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
    }

    async fn delete(&self, endpoint: &str) -> Result<Response, reqwest::Error> {
        self.client
            .delete(format!("{}{}", self.api_url, endpoint).as_str())
            .send()
            .await
    }
}
