use serde::{Deserialize, Serialize};

pub use ureq::{Error, ErrorKind};

pub type Result<T> = core::result::Result<T, ureq::Error>;

pub mod config {
    #[derive(Clone, Debug, PartialEq)]
    pub struct ApiConfig {
        pub minor_version: String,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct QueryConfig<'a> {
        pub r#where: Option<&'a str>,
        pub order_by: Option<&'a str>,
        pub start_position: usize,
        pub max_results: usize,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct QuickbooksConfig {
        pub client_id: String,
        pub client_secret: String,

        pub base_url: String,
        pub company_id: String,

        pub token: super::AccessToken,

        pub api: Option<ApiConfig>,
    }

    impl Default for QueryConfig<'_> {
        fn default() -> Self {
            Self {
                r#where: None,
                order_by: None,
                start_position: 1,
                max_results: crate::constants::MAX_QUERY_LENGTH,
            }
        }
    }
}

mod defaults {
    pub mod access_token {
        #[inline]
        pub fn token_type() -> String {
            "Bearer".to_string()
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct AccessToken {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access_token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub refresh_token: String,
    #[serde(
        default = "defaults::access_token::token_type",
        skip_serializing_if = "String::is_empty"
    )]
    pub token_type: String,
    //#[serde(default)]
    //pub expires_in: i64,
    //#[serde(default)]
    //pub x_refresh_token_expires_in: i64,
}
