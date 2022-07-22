use crate::{
    constants::MAX_QUERY_LENGTH,
    types::{config::*, *},
    Error,
};

use concat_string::concat_string;
use ureq::{Agent, AgentBuilder, Request};

type Result = crate::Result<ureq::Response>;

#[derive(Clone, Debug)]
struct ApiPaths {
    pub query: String,
}

#[derive(Clone, Debug)]
pub struct Quickbooks {
    config: QuickbooksConfig,

    agent: Agent,
    paths: ApiPaths,
}

impl From<QuickbooksConfig> for Quickbooks {
    fn from(config: QuickbooksConfig) -> Self {
        use std::time::Duration;

        let api_config = config.api.clone().unwrap_or(ApiConfig {
            minor_version: "65".into(),
        });

        let minor_version = concat_string!("?minorversion=", api_config.minor_version);

        let builder = AgentBuilder::new()
            .timeout(Duration::from_secs(5))
            .https_only(true);

        Self {
            config: config.clone(),
            agent: builder.build(),
            paths: ApiPaths {
                query: format!(
                    "{}/v3/company/{}/query{}",
                    config.base_url, config.company_id, minor_version
                ),
            },
        }
    }
}

impl Quickbooks {
    fn build_query(&self, query: &str) -> Request {
        // TODO: url encode `query`?
        self.agent
            .get(&self.paths.query)
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set(
                "Authorization",
                &concat_string!(
                    self.config.token.token_type,
                    " ",
                    self.config.token.access_token
                ),
            )
            .query("query", query)
    }

    fn query(&self, query: &str) -> Result {
        self.build_query(query).call()
    }

    pub fn company_info(&self) -> Result {
        self.query("SELECT * FROM CompanyInfo")
    }

    // TODO: return a vec of queries if necessary to get all the results?
    /// Returns a list of all items (products) up to 1,000 items
    pub fn list_items(&self) -> Result {
        self.query(&concat_string!(
            "SELECT * FROM Item MAXRESULTS ",
            MAX_QUERY_LENGTH.to_string()
        ))
    }

    /// this does not currently work, presumably due to a bug in ureq
    pub fn refresh_access_token(&mut self) -> core::result::Result<AccessToken, Error> {
        use http_auth_basic::Credentials;

        let credentials = Credentials::new(&self.config.client_id, &self.config.client_secret);

        let response = ureq::post("https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer")
            .set("accept", "application/json")
            .set("authorization", &credentials.as_http_header())
            .set("Content-Type", "application/x-www-form-urlencoded")
            .query("grant_type", "refresh_token")
            .query("refresh_token", &self.config.token.refresh_token)
            .call()?;

        let token: AccessToken = response.into_json()?;

        self.config.token = token.clone();

        Ok(token)
    }

    // TODO: remove once Self::refresh_access_token() works with ureq
    /// This will be removed once refresh_access_token() works with ureq
    pub fn refresh_access_token_with_reqwest(
        &mut self,
    ) -> core::result::Result<AccessToken, reqwest::Error> {
        use reqwest::header;

        let mut headers = header::HeaderMap::new();
        headers.append(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &self.config.token.refresh_token),
        ];
        let client = reqwest::blocking::Client::new();
        let response = client
            .post("https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer")
            .headers(headers)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()?;

        let token: AccessToken = response.json()?;

        self.config.token = token.clone();

        Ok(token)
    }
}
