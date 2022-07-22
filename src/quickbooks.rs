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
    pub read_item: String,
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

        let base = concat_string!(config.base_url, "/v3/company/", config.company_id);

        Self {
            config: config.clone(),
            agent: builder.build(),
            paths: ApiPaths {
                read_item: concat_string!(base, "/item/"),
                query:     concat_string!(base, "/query", minor_version),
            },
        }
    }
}

trait SetHeaders {
    fn set_headers(self, token: &AccessToken) -> Self;
}

impl SetHeaders for Request {
    fn set_headers(self, token: &AccessToken) -> Self {
        self
            .set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set(
                "Authorization",
                &concat_string!(
                    token.token_type,
                    " ",
                    token.access_token
                ),
            )
    }
}

impl Quickbooks {
    fn build_query(&self, query: &str) -> Request {
        // TODO: url encode `query`?
        self.agent
            .get(&self.paths.query)
            .set_headers(&self.config.token)
            .query("query", query)
    }

    fn query(&self, query: &str) -> Result {
        self.build_query(query).call()
    }

    pub fn company_info(&self) -> Result {
        self.query("SELECT * FROM CompanyInfo")
    }

    pub fn read_item(&self, item_id: &str) -> Result {
        self.agent
            .get(&concat_string!(self.paths.read_item, item_id, "?minorversion=65"))
            .set_headers(&self.config.token)
            .call()
    }

    // It doesn't seem to be possible to return a Vec<Response> in case there are
    // more than 1,000 items due to a limitation of ureq, so we're left with `start_position` :/

    /// Returns a list of all items (products) up to 1,000 items, starting at `start_position` (must be at least 1)
    pub fn list_items(&self, start_position: usize) -> Result {
        #[cfg(debug_assertions)]
        assert_ne!(start_position, 0);

        self.query(&concat_string!(
            "SELECT * FROM Item MAXRESULTS ",
            MAX_QUERY_LENGTH.to_string(),
            " STARTPOSITION ", start_position.to_string()
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
