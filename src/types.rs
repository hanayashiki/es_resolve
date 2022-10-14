use indexmap::IndexMap;
use serde::Deserialize;

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
    // TODO: add extensions option
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

/// Any errors that might occur during [`crate::EsResolver::resolve`]
#[derive(Debug)]
pub enum EsResolverError {
    /// Fail to resolve the target because unable to load a critical file,
    /// e.g. that [`crate::EsResolver::from`] is not a real file.
    IOError(std::io::Error, String),
    /// Fail to read a package.json. When `LOAD_PACKAGE_EXPORTS` is assumpted but
    /// the package.json is invalid, this is raised in accordance to Node's behavior.
    InvalidPackageJSON(serde_json::Error),
    /// Fail to read a tsconfig.json
    InvalidTSConfig(serde_json::Error),
    InvalidTSConfigExtend(String),
    /// When `LOAD_PACKAGE_EXPORTS`, the exports field is found invalid.
    /// See <https://nodejs.org/api/packages.html#subpath-exports>.
    InvalidExports(String),
    InvalidModuleSpecifier(String),
    ModuleNotFound(String),
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
#[serde(rename_all = "camelCase")]
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
    Array(Vec<Exports>),
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TSConfig {
    pub extends: Option<String>,
    #[serde(default)]
    pub compiler_options: TSConfigCompilerOptions,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TSConfigCompilerOptions {
    pub base_url: Option<String>,
    pub paths: Option<TSConfigPaths>,
}

impl Default for TSConfigCompilerOptions {
    fn default() -> Self {
        TSConfigCompilerOptions {
            base_url: None,
            paths: None,
        }
    }
}

pub type TSConfigPaths = IndexMap<String, Vec<String>>;
