use reqwest::{header, Response, StatusCode};
use thiserror::Error;

pub mod account;
pub mod token;
pub mod domain;
pub mod rrset;

pub const API_URL: &str = "https://desec.io/api/v1";

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
    #[error("The given credentials are not valid")]
    Forbidden,
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
    #[error("Request is unauthorized: {0}")]
    Unauthorized(String),
}

#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
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
            client
        })
    }

    /// Creates a new client using the given credentials.
    ///
    /// # Errors
    ///
    /// This method fails with [`Error::ReqwestClientBuilder`][error] if the underlying [`reqwest::ClientBuilder`][builder] fails to build a http client.
    ///
    /// [error]: enum.Error.html
    /// [builder]: https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html#method.build
    pub async fn new_from_credentials(email: &str, password: &str) -> Result<Self, Error> {
        let login = account::login(email, password).await?;
        Client::new(login.token)
    }

    async fn get(&self, endpoint: &str) -> Result<Response, reqwest::Error> {
        self.client
            .get(format!("{}{}", API_URL, endpoint))
            .send()
            .await
    }

    async fn post(&self, endpoint: &str, body: Option<String>) -> Result<Response, reqwest::Error> {
        self.client
            .post(format!("{}{}", API_URL, endpoint).as_str())
            .header("Content-Type", "application/json")
            .body(body.unwrap_or_default()) // body is optional, so we send empty string when None
            .send()
            .await
    }

    async fn patch(&self, endpoint: &str, body: String) -> Result<Response, reqwest::Error> {
        self.client
            .patch(format!("{}{}", API_URL, endpoint).as_str())
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
    }

    async fn delete(&self, endpoint: &str) -> Result<Response, reqwest::Error> {
        self.client
            .delete(format!("{}{}", API_URL, endpoint).as_str())
            .send()
            .await
    }
}

async fn process_response_error(response: reqwest::Response) -> Error {
    match response.status() {
        StatusCode::UNAUTHORIZED => {
            Error::Unauthorized(
                response.text().await.unwrap_or_default(),
            )
        },
        StatusCode::FORBIDDEN => {
            Error::Forbidden
        },
        StatusCode::BAD_REQUEST => {
            Error::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default(),
            )
        },
        StatusCode::NOT_FOUND => {
            Error::NotFound
        },
        StatusCode::TOO_MANY_REQUESTS => {
            match response.headers().get("retry-after") {
                Some(header) => match header.to_str() {
                    Ok(header) => Error::RateLimited(
                        header.to_string(),
                        response.text().await.unwrap_or_default(),
                    ),
                    Err(_) => Error::ApiError(
                        response.status().into(),
                        "Request got throttled with invalid retry-after header".to_string(),
                    ),
                },
                None => Error::ApiError(
                    response.status().into(),
                    "Request got throttled without retry-header".to_string(),
                ),
            }
        },
        _ => Error::UnexpectedStatusCode(
            response.status().into(),
            response.text().await.unwrap_or_default(),
        )
    }
}
