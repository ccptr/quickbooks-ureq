pub mod base_url {
    pub const PRODUCTION: &str = "https://quickbooks.api.intuit.com";
    pub const SANDBOX: &str = "https://sandbox-quickbooks.api.intuit.com";

    pub mod payments_api {
        pub const PRODUCTION: &str = "https://api.intuit.com";
        pub const SANDBOX: &str = "https://sandbox.api.intuit.com/quickbooks/v4/payments";
    }
}

pub const MAX_QUERY_LENGTH: usize = 1000;
