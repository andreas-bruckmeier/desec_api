use serde::{Deserialize, Serialize};
use reqwest::StatusCode;
use crate::{Client, Error};

pub struct AccountClient<'a> {
    pub(crate) client: &'a crate::Client,
}

impl<'a> Client {
    pub fn account(&'a self) -> AccountClient<'a> {
        AccountClient { client: self }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountInformation {
    pub created: String,
    pub email: String,
    pub id: String,
    pub limit_domains: u64,
    pub outreach_preference: bool,
}

impl<'a> AccountClient<'a> {
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
