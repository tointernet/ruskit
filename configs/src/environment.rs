use std::{
    env,
    fmt::{Display, Formatter, Result},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Environment {
    #[default]
    Local,
    Dev,
    Staging,
    Prod,
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let printable = match *self {
            Environment::Local => "local",
            Environment::Dev => "dev",
            Environment::Staging => "stg",
            Environment::Prod => "prd",
        };
        write!(f, "{}", printable)
    }
}

impl Environment {
    pub fn from_rust_env() -> Environment {
        let env = env::var("RUST_ENV").unwrap_or_default();

        match env.as_str() {
            "production" | "PRODUCTION" | "prod" | "PROD" | "prd" | "PRD" => Environment::Prod,
            "staging" | "STAGING" | "stg" | "STG" => Environment::Staging,
            "develop" | "DEVELOP" | "dev" | "DEV" => Environment::Dev,
            _ => Environment::Local,
        }
    }

    pub fn is_local(&self) -> bool {
        self == &Environment::Local
    }

    pub fn is_dev(&self) -> bool {
        self == &Environment::Dev
    }

    pub fn is_stg(&self) -> bool {
        self == &Environment::Staging
    }

    pub fn is_prod(&self) -> bool {
        self == &Environment::Prod
    }
}
