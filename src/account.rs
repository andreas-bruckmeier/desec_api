use serde::{Deserialize, Serialize};
use reqwest::StatusCode;
use crate::{Client, Error};

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
        match self.client.get("/auth/account/").await {
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
}
