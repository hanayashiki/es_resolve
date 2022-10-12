use indexmap::IndexMap;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum MainFields {
    Main,
    Module,
    Browser,
    ReactNative,
}

#[derive(Clone, Debug)]
pub enum TargetEnv {
    Node,
    Browser,
}

#[derive(Clone, Debug)]
pub struct EsResolveOptions {
    pub main_fields: Vec<MainFields>,
    pub conditions: Vec<String>,
}

impl EsResolveOptions {
    pub fn default_for(env: TargetEnv) -> Self {
        match env {
            TargetEnv::Node => Self {
                main_fields: vec![MainFields::Main, MainFields::Module], // Node.js itself doesn't respect "module"
                conditions: vec![format!("node"), format!("require"), format!("default")],
            },
            TargetEnv::Browser => Self {
                main_fields: vec![MainFields::Browser, MainFields::Module, MainFields::Main],
                conditions: vec![format!("browser"), format!("import"), format!("default")],
            },
        }
    }
}

#[derive(Debug)]
pub enum EsResolverError {
    IOError(std::io::Error, String),
    InvalidPackageJSON(serde_json::Error),
    InvalidExports(String),
    InvalidModuleSpecifier(String)
}

pub type EsResolverResult<T> = Result<T, EsResolverError>;

#[derive(Debug)]
pub enum Extensions {
    Mjs,
    Mts,
    Cjs,
    Cts,
    Json,
    Js,
    Jsx,
    Ts,
    Tsx,
    Node,
    Css,
}

impl Extensions {
    pub fn from(ext: &str) -> Option<Extensions> {
        match ext {
            "mjs" => Some(Extensions::Mjs),
            "mts" => Some(Extensions::Mts),
            "cjs" => Some(Extensions::Cjs),
            "cts" => Some(Extensions::Cts),
            "json" => Some(Extensions::Json),
            "js" => Some(Extensions::Js),
            "jsx" => Some(Extensions::Jsx),
            "ts" => Some(Extensions::Ts),
            "tsx" => Some(Extensions::Tsx),
            "node" => Some(Extensions::Node),
            "css" => Some(Extensions::Css),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Extensions::Mjs => "mjs",
            Extensions::Mts => "mts",
            Extensions::Cjs => "cjs",
            Extensions::Cts => "cts",
            Extensions::Json => "json",
            Extensions::Js => "js",
            Extensions::Jsx => "jsx",
            Extensions::Ts => "ts",
            Extensions::Tsx => "tsx",
            Extensions::Node => "node",
            Extensions::Css => "css",
        }
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PackageJSON {
    pub main: Option<String>,
    pub module: Option<String>,
    pub browser: Option<String>,
    pub react_native: Option<String>,
    pub exports: Option<Exports>,
}

impl PackageJSON {
    pub fn get_main_field(&self, field: &MainFields) -> Option<String> {
        match field {
            MainFields::Main => self.main.clone(),
            MainFields::Module => self.module.clone(),
            MainFields::Browser => self.browser.clone(),
            MainFields::ReactNative => self.react_native.clone(),
        }
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Exports {
    String(String),
    Object(IndexMap<String, Option<Exports>>),
    Array(Vec<Exports>)
}
