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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub token: String,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptchaKind {
    Image,
}

impl<'a> AccountClient<'a> {
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
        let response = self.client.get("/auth/account/").await?;
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

    /// Updates the accounts outreach preference, the only field currently updatable.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn update_outreach_preference(
        &self,
        outreach_preference: bool,
    ) -> Result<AccountInformation, Error> {
        let response = self
            .client
            .patch(
                "/auth/account/",
                json!({"outreach_preference": outreach_preference}).to_string(),
            )
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

    /// Initiates a password reset using your email address and a captcha solution.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn request_password_reset(
        &self,
        email: &str,
        captcha_id: &str,
        captcha_solution: &str,
    ) -> Result<AccountInformation, Error> {
        let response = self
            .client
            .post(
                "/auth/account/reset-password/",
                Some(
                    json!({
                      "email": email,
                      "captcha": {
                        "id": captcha_id,
                        "solution": captcha_solution
                      }
                    })
                    .to_string(),
                ),
            )
            .await?;
        match response.status() {
            StatusCode::ACCEPTED => {
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

    /// Confirms a password reset using the code sent via email.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn confirm_password_reset(
        &self,
        new_password: &str,
        code: &str,
    ) -> Result<AccountInformation, Error> {
        let response = self
            .client
            .post(
                format!("/auth/account/reset-password/{code}").as_str(),
                Some(json!({"new_password": new_password}).to_string()),
            )
            .await?;
        match response.status() {
            StatusCode::ACCEPTED => {
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

    /// Updates your accounts email address.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn update_email(
        &self,
        email: &str,
        password: &str,
        new_email: &str,
    ) -> Result<AccountInformation, Error> {
        let response = self
            .client
            .post(
                "/auth/account/change-email/",
                Some(
                    json!({
                      "email": email,
                      "password": password,
                      "new_email": new_email
                    })
                    .to_string(),
                ),
            )
            .await?;
        match response.status() {
            StatusCode::ACCEPTED => {
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

    /// Deletes your account.
    ///
    /// Before you can delete your account, it is required to first delete all your domains from deSEC.
    ///
    /// # Errors
    ///
    /// This method fails with:
    /// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
    /// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
    /// - [`Error::Reqwest`][error] if the whole request failed
    ///
    /// [error]: ../enum.Error.html
    pub async fn delete_account(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AccountInformation, Error> {
        let response = self
            .client
            .post(
                "/auth/account/delete/",
                Some(json!({"email": email, "password": password}).to_string()),
            )
            .await?;
        match response.status() {
            StatusCode::ACCEPTED => {
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
pub async fn get_captcha() -> Result<Captcha, Error> {
    let client =
        Client::new_unauth().map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
    let response = client.post("/captcha/", None).await?;
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
    email: &str,
    password: &str,
    captcha_id: &str,
    captcha_solution: &str,
    domain: Option<&str>,
) -> Result<RegisterResponse, Error> {
    let payload = if let Some(domain) = domain {
        json!({
            "email": email,
            "password": password,
            "captcha": {
                "id": captcha_id,
                "solution": captcha_solution
            },
            "domain": domain
        })
        .to_string()
    } else {
        json!({
            "email": email,
            "password": password,
            "captcha": {
                "id": captcha_id,
                "solution": captcha_solution
            }
        })
        .to_string()
    };
    let client =
        Client::new_unauth().map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
    let response = client.post("/auth/", Some(payload)).await?;
    match response.status() {
        StatusCode::ACCEPTED => {
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

/// Performs a login request using the given credentials and returns the login information.
///
/// # Errors
///
/// This method fails with:
/// - [`Error::InvalidAPIResponse`][error] if the response cannot be parsed into desec_api::rrset::ResourceRecordSet
/// - [`Error::UnexpectedStatusCode`][error] if the API responds with an undocumented status code
/// - [`Error::Reqwest`][error] if the whole request failed
///
/// [error]: ../enum.Error.html
pub async fn login(email: &str, password: &str) -> Result<Login, Error> {
    let client =
        Client::new_unauth().map_err(|error| Error::ReqwestClientBuilder(error.to_string()))?;
    let response = client
        .post(
            "/auth/login/",
            Some(
                json!({
                    "email": email,
                    "password": password,
                })
                .to_string(),
            ),
        )
        .await?;
    match response.status() {
        StatusCode::OK => {
            // Build the final client using the token from the login
            let response_text = response.text().await.map_err(Error::Reqwest)?;
            Ok(serde_json::from_str(&response_text)
                .map_err(|error| Error::InvalidAPIResponse(error.to_string(), response_text))?)
        }
        _ => Err(Error::UnexpectedStatusCode(
            response.status().into(),
            response.text().await.unwrap_or_default(),
        )),
    }
}
