//! Unofficial Rust client for the [deSEC DNS API](https://desec.readthedocs.io/en/latest/).
//!
//! # Supported endpoints
//!
//! * Manage accounts
//!   * Obtain a Captcha
//!   * Register Account with optional domain creation
//!   * Log In (Retrieve API token using email & password)
//!   * Log Out (When client was created from credentials)
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
//!   * Retrieving all RRsets in a Zone filtered by type
//!   * Retrieving all RRsets in a Zone filtered by subname
//!   * Retrieving a Specific RRset
//!   * Modifying an RRset
//!   * Deleting an RRset
//!
//! * Manage Tokens
//!   * Create a token
//!   * Modify a token
//!   * List all tokens
//!   * Retrieve a specific token
//!   * Delete a token
//!
//! * Manage Token Policies
//!   * Create a token policy (including default policy)
//!   * Modify a token policy
//!   * List all token policies
//!   * Delete a token policy
//!
//! # Currently not supported
//!
//! * Pagination when over 500 items exist
//! * Manage DNS records
//!   * Bulk operations when modifying or deleting RRsets
//!
//! # General errors for all clients
//!
//! There are some error which can occure for every client (account, domain, rrset, token).
//!
//! This method fails with:
//! - [`Error::Reqwest`][error] if there was a problem in the underlying http client
//! - [`Error::Unauthorized`][error] if the token of the client is invalid
//! - [`Error::Forbidden`][error] if you are not allow to access a resource
//! - [`Error::RateLimitedMaxRetriesReached`][error] if a request has been throttled too many times
//! - [`Error::ApiError`][error] if the deSEC response cannot be transformed in the expected type
//! - [`Error::NotFound`][error] if the resource does not exist
//! - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
//! - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
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
//!
//! [error]: enum.Error.html

use const_format::concatcp;
use log::debug;
use reqwest::{header, Response, StatusCode};
use thiserror::Error;
use tokio::time::{sleep, Duration};

pub mod account;
pub mod domain;
pub mod rrset;
pub mod token;

pub const API_URL: &str = "https://desec.io/api/v1";

// Build useragent at compile time
pub const USERAGENT: &str = concatcp!(
    "desec-api-client/",
    env!("CARGO_PKG_VERSION"),
    " (unoffical deSEC API client written in Rust)"
);

#[derive(Error, Debug)]
pub enum Error {
    #[error("An error occurred during the request")]
    Reqwest(reqwest::Error),
    #[error("You hit a rate limit and need to wait {0} seconds. Additional Info: {1}")]
    RateLimited(u64, String),
    #[error("You hit a rate limit and need to wait. Additional Info: {0}")]
    RateLimitedWithoutRetry(String),
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
    #[error("Client has not been logged in, so you cannot logout")]
    CannotLogout,
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
    /// Whether this client has been logged in before
    logged_in: bool,
}

impl Client {
    fn get_client(token: Option<String>, logged_in: Option<bool>) -> Result<Self, Error> {
        let mut client = reqwest::ClientBuilder::new().user_agent(USERAGENT);
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
            logged_in: logged_in.unwrap_or_default(),
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
        Client::get_client(Some(token), None)
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
        Client::get_client(Some(login.token), Some(true))
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
        Client::get_client(None, None)
    }

    /// Consume and logout the authenticated client.
    ///
    /// Attention: this assumes that the client has been authenticated using credentials.
    /// Trying to logout a client created from a token will return Error::CannotLogout.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::CannotLogout`][error] if the client was not created from credentials
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn logout(self) -> Result<(), Error> {
        // No logout for clients no logged in
        if !self.logged_in {
            return Err(Error::CannotLogout);
        }
        let response = self.post("/auth/logout/", None).await?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            _ => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
        }
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
                Ok(response) => match response.status() {
                    StatusCode::OK
                    | StatusCode::CREATED
                    | StatusCode::NO_CONTENT
                    | StatusCode::ACCEPTED => return Ok(response),
                    StatusCode::TOO_MANY_REQUESTS => {
                        let ttw =
                            parse_time_to_wait(response, self.max_wait_retry, self.retry).await?;
                        debug!("Request has been throttled, we wait {} seconds", ttw);
                        sleep(Duration::from_secs(ttw)).await;
                        retries += 1;
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
            .map_err(Error::Reqwest)?;
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
            .map_err(Error::Reqwest)?;
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
            .map_err(Error::Reqwest)?;
        self.process_request(request).await
    }

    /// Process delete requests
    async fn delete(&self, endpoint: &str) -> Result<Response, Error> {
        let request = self
            .client
            .delete(format!("{}{}", API_URL, endpoint).as_str())
            .build()
            .map_err(Error::Reqwest)?;
        self.process_request(request).await
    }
}

// Parsing the time we have to wait till next retry.
// Error out if we cannot parse, retry is disabled, or accepted max wait time will be exceeded.
async fn parse_time_to_wait(
    response: Response,
    max_wait_retry: u64,
    should_retry: bool,
) -> Result<u64, Error> {
    let time_to_wait = match response.headers().get("retry-after") {
        Some(header) => match header.to_str() {
            Ok(header) => header.parse().map_err(|_| {
                Error::RateLimitedWithoutRetry(format!(
                    "Request was throttled and cannot parse retry after {:?}",
                    header
                ))
            })?,
            Err(_) => return Err(Error::RateLimitedWithoutRetry(
                "Request got throttled with retry-after header containing non-visible ASCII chars"
                    .to_string(),
            )),
        },
        None => {
            return Err(Error::RateLimitedWithoutRetry(
                "Request got throttled without retry-after header".to_string(),
            ))
        }
    };
    // Abort if we are not interested in retries
    if !should_retry {
        let msg = String::from("Request has been throttled, but retries are disabled");
        debug!("{}", msg);
        return Err(Error::RateLimited(
            time_to_wait,
            response.text().await.unwrap_or(msg),
        ));
    }
    if time_to_wait > max_wait_retry {
        let msg = format!(
            "Wait time for retry {} exceeds max accepted wait time per retry {}",
            time_to_wait, max_wait_retry
        );
        debug!("{}", msg);
        return Err(Error::RateLimited(time_to_wait, msg));
    }
    Ok(time_to_wait)
}
