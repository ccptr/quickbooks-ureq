use crate::{
    constants::MAX_QUERY_LENGTH,
    types::{config::*, *},
    Error,
};

use concat_string::concat_string;
use ureq::{Agent, AgentBuilder, Request};

#[derive(Clone, Debug)]
struct ApiPaths {
    pub base: String,
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
            config,
            agent: builder.build(),
            paths: ApiPaths {
                query: concat_string!(base, "/query", minor_version),
                base,
            },
        }
    }
}

trait SetHeaders {
    fn set_headers(self, token: &AccessToken) -> Self;
}

impl SetHeaders for Request {
    fn set_headers(self, token: &AccessToken) -> Self {
        self.set("Accept", "application/json")
            .set("Content-Type", "application/json")
            .set(
                "Authorization",
                &concat_string!(token.token_type, " ", token.access_token),
            )
    }
}

impl Quickbooks {
    pub fn build_query(&self, query: &str) -> Request {
        // TODO: url encode `query`?
        self.agent
            .get(&self.paths.query)
            .set_headers(&self.config.token)
            .query("query", query)
    }

    pub fn query(&self, key: &str, config: &QueryConfig) -> Result {
        #[cfg(debug_assertions)]
        assert_ne!(config.start_position, 0);

        let mut query = concat_string!(
            "SELECT * FROM ", key, " MAXRESULTS ",
            MAX_QUERY_LENGTH.to_string(),
            " STARTPOSITION ",
            config.start_position.to_string()
        );

        if let Some(r#where) = config.r#where {
            query = concat_string!(query, " WHERE ", r#where);
        }

        if let Some(order_by) = config.order_by {
            query = concat_string!(query, " ORDERBY ", order_by);
        }

        self.build_query(&query).call()
    }

    /// reads a single item
    pub fn read(&self, key: &str, id: &str) -> Result {
        self.agent
            .get(&concat_string!(
                self.paths.base,
                "/", key, "/",
                id,
                "?minorversion=65"
            ))
            .set_headers(&self.config.token)
            .call()
    }

    pub fn company_info(&self) -> Result {
        self.build_query("SELECT * FROM CompanyInfo").call()
    }

    pub fn read_item(&self, id: &str) -> Result {
        self.read("item", id)
    }

    pub fn query_customers(&self, config: &QueryConfig) -> Result {
        self.query("Customer", config)
    }

    // It doesn't seem to be possible to return a Vec<Response> in case there are
    // more than 1,000 items due to a limitation of ureq, so we're left with `QueryConfig::start_position` :/

    /// Returns a list of items (products) up to 1,000 items, starting at `QueryConfig::start_position` (must be at least 1)
    pub fn query_items(&self, config: &QueryConfig) -> Result {
        self.query("Item", config)
    }

    /// this does not currently work, presumably due to a bug in ureq
    pub fn refresh_access_token(&mut self) -> core::result::Result<AccessToken, Error> {
        use http_auth_basic::Credentials;

        let credentials = Credentials::new(&self.config.client_id, &self.config.client_secret);

        // TODO: once ureq is fixed this can probably be changed to self.agent.post(...)...
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
