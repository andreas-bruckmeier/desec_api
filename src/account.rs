use crate::{Client, Error};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// An asynchronous client to work with the deSEC account API.
pub struct AccountClient<'a> {
    pub(crate) client: &'a crate::Client,
}

impl<'a> Client {
    /// Returns a wrapping client for the account API.
    pub fn account(&'a self) -> AccountClient<'a> {
        AccountClient { client: self }
    }
}

/// Representation of a deSEC [`account`][reference].
///
/// [reference]: https://desec.readthedocs.io/en/latest/auth/account.html
#[derive(Serialize, Deserialize, Debug)]
pub struct AccountInformation {
    pub created: String,
    pub email: String,
    pub id: String,
    pub limit_domains: u64,
    pub outreach_preference: bool,
}

/// Representation of a deSEC [`login`][reference].
///
/// [reference]: https://desec.readthedocs.io/en/latest/auth/account.html#log-in
#[derive(Serialize, Deserialize, Debug)]
pub struct Login {
    pub allowed_subnets: Vec<String>,
    pub created: String,
    pub is_valid: bool,
    pub last_used: Option<String>,
    pub max_age: String,
    pub max_unused_period: String,
    pub name: String,
    pub perm_manage_tokens: bool,
    pub token: String
}

/// Representation of a deSEC [`register`][reference] response.
///
/// [reference]: https://desec.readthedocs.io/en/latest/auth/account.html#register-account
#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterResponse {
    pub detail: String,
}

/// Representation of a deSEC [`captcha`][reference].
///
/// [reference]: https://desec.readthedocs.io/en/latest/auth/account.html#obtain-a-captcha
#[derive(Serialize, Deserialize, Debug)]
pub struct Captcha {
    pub id: String,
    pub challenge: String,
    pub kind: CaptchaKind,
}

/// Kind of challenge. Currently only image implemented.
///
/// [reference]: https://desec.readthedocs.io/en/latest/auth/account.html#obtain-a-captcha
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptchaKind {
    Image,
}

impl<'a> AccountClient<'a> {
    /// Retrieves a base64 encoded captcha neccessary to register a new Account
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn get_captcha(&self) -> Result<Captcha, Error> {
        match self.client.post("/captcha/", None).await {
            Ok(response) if response.status() == StatusCode::CREATED => {
                let response_text = response.text().await.map_err(Error::Reqwest)?;
                serde_json::from_str(&response_text)
                    .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))
            }
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }

    /// Registers a new account using a captcha solution, a capture id and an optional first domain.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn register(
        &self,
        email: &str,
        password: &str,
        captcha_id: &str,
        captcha_solution: &str,
        domain: Option<&str>,
    ) -> Result<RegisterResponse, Error> {
        let payload = if let Some(domain) = domain {
            Some(
                json!({
                    "email": email,
                    "password": password,
                    "captcha": {
                        "id": captcha_id,
                        "solution": captcha_solution
                    },
                    "domain": domain
                })
                .to_string(),
            )
        } else {
            Some(
                json!({
                    "email": email,
                    "password": password,
                    "captcha": {
                        "id": captcha_id,
                        "solution": captcha_solution
                    }
                })
                .to_string(),
            )
        };
        match self.client.post("/auth/", payload).await {
            Ok(response) if response.status() == StatusCode::ACCEPTED => {
                let response_text = response.text().await.map_err(Error::Reqwest)?;
                serde_json::from_str(&response_text)
                    .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))
            }
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

    /// Performs a login using username and password and returns an authenticated desec_api::Client on success.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::account::Login
    /// - [`Error::Forbidden`][error] in case of credential mismatch
    /// - [`Error::UnexpectedStatusCode`][error] in case of the API responds with an undocumented status code
    /// - [`Error::ReqwestClientBuilder`][error] in case building the authenticated client fails
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Client, Error> {
        let login = self.login(email, password).await?;
        Ok(Client::new(login.token)?)
    }

    /// Performs a login using username and password and returns a desec_api::account::Login on success.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::account::Login
    /// - [`Error::Forbidden`][error] in case of credential mismatch
    /// - [`Error::UnexpectedStatusCode`][error] in case of the API responds with an undocumented status code
    /// - [`Error::ReqwestClientBuilder`][error] in case building the authenticated client fails
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Login, Error> {
        let payload = json!({
            "email": email,
            "password": password,
        })
        .to_string();
        match self.client.post("/auth/login/", Some(payload)).await {
            Ok(response) if response.status() == StatusCode::OK => {
                let response_text = response.text().await.map_err(Error::Reqwest)?;
                serde_json::from_str(&response_text)
                    .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))
            }
            Ok(response) if response.status() == StatusCode::FORBIDDEN => Err(Error::Forbidden),
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }

    /// Retrieves the account information.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn get_account_info(&self) -> Result<AccountInformation, Error> {
        match self.client.get("/auth/account/").await {
            Ok(response) if response.status() == StatusCode::OK => {
                let response_text = response.text().await.map_err(Error::Reqwest)?;
                serde_json::from_str(&response_text)
                    .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))
            }
            Ok(response) => Err(Error::UnexpectedStatusCode(
                response.status().into(),
                response.text().await.unwrap_or_default(),
            )),
            Err(error) => Err(Error::Reqwest(error)),
        }
    }
}
