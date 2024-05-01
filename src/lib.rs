//! Unofficial Rust client for the [deSEC DNS API](https://desec.readthedocs.io/en/latest/).
//!
//! # Supported endpoints
//!
//! * Manage accounts
//!   * Obtain a Captcha
//!   * Register Account with optional domain creation
//!   * Log In (Retrieve API token using email & password)
//!   * Retrieve account information
//!   * Modify account settings (only updating outreach_preference is supported by the API)
//!   * Password reset (Request for password reset & confirmation, but handling of approval via mail needs to be handled)
//!   * Password change
//!   * Change of email address
//!   * Delete account
//!
//! * Manage domains
//!   * Creating a domain
//!   * List domains
//!   * Retrieve a specific domain
//!   * Identifying the responsible domain for a DNS name
//!   * Exporting a domain as zonefile
//!   * Deleting a domain
//!
//! * Manage DNS records
//!   * Creating an RRset
//!   * Retrieving all RRsets in a Zone
//!   * Retrieving a Specific RRset
//!   * Modifying an RRset
//!   * Deleting an RRset
//!
//! # Currently not supported
//!
//! * Pagination when over 500 items exist
//! * Account
//!   * Logout: login tokens expire 7 days after creation or when not used for 1 hour, whichever comes first.
//!       But maybe a logout function will be added in future version.
//! * Manage DNS records
//!   * Filtering when retrieving RRsets
//!   * Bulk operations when modifying or deleting RRsets
//!
//! # Usage example
//!
//! ## With existing API token
//! ```no_run
//!use desec_api::Client;
//!
//!#[tokio::main]
//!async fn main() {
//!
//!    let client = Client::new("i-T3b1h_OI-H9ab8tRS98stGtURe".to_string())
//!        .unwrap();
//!
//!    // Retrieve account informations
//!    let account_info = client
//!        .account()
//!        .get_account_info()
//!        .await
//!        .unwrap();
//!    
//!    println!("{:#?}", account_info);
//!}
//! ```
//!
//! ## With login credentials
//! ```no_run
//!use desec_api::Client;
//!
//!#[tokio::main]
//!async fn main() {
//!
//!    let client = Client::new_from_credentials("info@example.com", "mysecret")
//!        .await
//!        .unwrap();
//!
//!    // Retrieve all RRsets of domain `example.com`
//!    let rrsets = client
//!        .rrset()
//!        .get_rrsets("example.com")
//!        .await
//!        .unwrap();
//!    
//!    println!("{:#?}", rrsets);
//!}
//! ```

use log::debug;
use reqwest::{header, Response, StatusCode};
use thiserror::Error;
use tokio::time::{sleep, Duration};

pub mod account;
pub mod domain;
pub mod rrset;
pub mod token;

pub const API_URL: &str = "https://desec.io/api/v1";

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

#[derive(Error, Debug)]
pub enum Error {
    #[error("An error occurred during the request")]
    Reqwest(reqwest::Error),
    // Could be integer but not header also allows http-dates
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Retry-After
    #[error("You hit a rate limit and need to wait {0} seconds. Additional Info: {1}")]
    RateLimited(String, String),
    #[error("The maximum count of retries has been reached")]
    RateLimitedMaxRetriesReached,
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
    /// Wheter to retry throttled requests based on the retry header
    retry: bool,
    /// Maximum waiting time to accept on a single retry
    max_wait_retry: u64,
    /// Maximum number of retries
    max_retries: usize,
}

fn get_useragent() -> String {
    format!(
        "desec-api-client/{} (unoffical deSEC API client written in Rust)",
        VERSION.unwrap_or("unknown")
    )
}

impl Client {
    fn get_client(token: Option<String>) -> Result<Self, Error> {
        let mut client = reqwest::ClientBuilder::new().user_agent(get_useragent());
        if let Some(token) = token {
            let mut headers = header::HeaderMap::new();
            headers.insert(
                "Authorization",
                header::HeaderValue::from_str(format!("Token {}", token.as_str()).as_str())
                    .unwrap(),
            );
            client = client.default_headers(headers);
        }
        let client = client
            .build()
            .map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
        Ok(Client {
            client,
            retry: true,
            max_wait_retry: 60,
            max_retries: 3,
        })
    }

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
        Client::get_client(Some(token))
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
        Client::get_client(Some(login.token))
    }

    /// Creates a new unauthenticated client for (captcha, register, login, e.g.).
    ///
    /// # Errors
    ///
    /// This method fails with [`Error::ReqwestClientBuilder`][error] if the underlying [`reqwest::ClientBuilder`][builder] fails to build a http client.
    ///
    /// [error]: enum.Error.html
    /// [builder]: https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html#method.build
    fn new_unauth() -> Result<Self, Error> {
        Client::get_client(None)
    }

    /// Sets whether retries are enabled.
    pub fn set_retry(&mut self, retry: bool) {
       self.retry = retry;
    }

    /// Returns whether retries are enabled.
    pub fn get_retry(&self) -> &bool {
       &self.retry
    }

    /// Sets the maximum wait time for a single retry
    pub fn set_max_wait_retry(&mut self, max_wait_retry: u64) {
       self.max_wait_retry = max_wait_retry;
    }

    /// Returns the maximum wait time for a single retry
    pub fn get_max_wait_retry(&self) -> &u64 {
       &self.max_wait_retry
    }

    /// Sets the maximum number of retries
    pub fn set_max_retries(&mut self, max_retries: usize) {
       self.max_retries = max_retries;
    }

    /// Returns the maximum number of retries
    pub fn get_max_retries(&self) -> &usize {
       &self.max_retries
    }

    /// Sends the request and processes the response.
    /// If a status code 429 is encountered, depending on the configuration, retries are done.
    async fn process_request(&self, request: reqwest::Request) -> Result<Response, Error> {
        let mut retries: usize = 0;
        loop {
            // We reached max retry limit, so we abort
            if retries > self.max_retries {
                debug!("Giving up after {} retries", self.max_retries);
                return Err(Error::RateLimitedMaxRetriesReached);
            }
            // Clone and execute the request.
            // Cloning should never fail because we have to streamed body or
            // other surprises.
            let result = self
                .client
                .execute(
                    request
                        .try_clone()
                        .expect("this request should always be clonable"),
                )
                .await;
            match result {
                // Retry logic
                Ok(response) => match response.status() {
                    StatusCode::OK
                    | StatusCode::CREATED
                    | StatusCode::NO_CONTENT
                    | StatusCode::ACCEPTED => return Ok(response),
                    StatusCode::TOO_MANY_REQUESTS => {
                        // Early abort if we are not interested in retries
                        if !self.retry {
                            debug!("Request has been throttled, but retries are disabled");
                            return Err(Error::ApiError(
                                response.status().as_u16(),
                                response.text().await.unwrap_or_default(),
                            ));
                        }
                        match response.headers().get("retry-after") {
                            Some(header) => match header.to_str() {
                                Ok(header) => {
                                    let time_to_wait: u64 = header.parse().map_err(|_| Error::ApiError(response.status().as_u16(), "foo".to_string()))?;
                                    if time_to_wait > self.max_wait_retry {
                                        debug!("Wait time for retry {} exceeds max accepted wait time per retry {}", time_to_wait, self.max_wait_retry);
                                        return Err(Error::ApiError(
                                            response.status().into(),
                                            format!("Wait time for retry {} exceeds max accepted wait time per retry {}", time_to_wait, self.max_wait_retry),
                                        ));
                                    }
                                    debug!("Request has been throttled, we wait {} seconds", time_to_wait);
                                    sleep(Duration::from_secs(time_to_wait)).await;
                                    retries += 1;
                                },
                                Err(_) => return Err(Error::ApiError(
                                    response.status().into(),
                                    "Request got throttled with retry-after header containing von ASCII chars".to_string(),
                                )),
                            },
                            None => return Err(Error::ApiError(
                                response.status().into(),
                                "Request got throttled without retry-header".to_string(),
                            ))
                        }
                    }
                    StatusCode::UNAUTHORIZED => {
                        return Err(Error::Unauthorized(
                            response.text().await.unwrap_or_default(),
                        ))
                    }
                    StatusCode::FORBIDDEN => return Err(Error::Forbidden),
                    StatusCode::BAD_REQUEST => {
                        return Err(Error::ApiError(
                            response.status().as_u16(),
                            response.text().await.unwrap_or_default(),
                        ))
                    }
                    StatusCode::NOT_FOUND => return Err(Error::NotFound),
                    _ => {
                        return Err(Error::UnexpectedStatusCode(
                            response.status().into(),
                            response.text().await.unwrap_or_default(),
                        ))
                    }
                },
                // Maybe retry on reqwest errors too?
                Err(error) => return Err(Error::Reqwest(error)),
            }
        }
    }

    /// Process get requests
    async fn get(&self, endpoint: &str) -> Result<Response, Error> {
        let request = self
            .client
            .get(format!("{}{}", API_URL, endpoint))
            .build()
            .map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
        self.process_request(request).await
    }

    /// Process post requests
    async fn post(&self, endpoint: &str, body: Option<String>) -> Result<Response, Error> {
        let request = self
            .client
            .post(format!("{}{}", API_URL, endpoint).as_str())
            .header("Content-Type", "application/json")
            .body(body.unwrap_or_default()) // body is optional, so we send empty string when None
            .build()
            .map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
        self.process_request(request).await
    }

    /// Process patch requests
    async fn patch(&self, endpoint: &str, body: String) -> Result<Response, Error> {
        let request = self
            .client
            .patch(format!("{}{}", API_URL, endpoint).as_str())
            .header("Content-Type", "application/json")
            .body(body)
            .build()
            .map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
        self.process_request(request).await
    }

    /// Process delete requests
    async fn delete(&self, endpoint: &str) -> Result<Response, Error> {
        let request = self
            .client
            .delete(format!("{}{}", API_URL, endpoint).as_str())
            .build()
            .map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
        self.process_request(request).await
    }
}
