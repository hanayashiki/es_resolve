use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{data::*, types::*};

pub struct EsResolver<'a> {
    target: &'a str,
    from: &'a PathBuf,
    env: TargetEnv,
    options: NodeResolveOptions,
}

impl<'a> EsResolver<'a> {
    pub fn new(target: &'a str, from: &'a PathBuf, env: TargetEnv) -> Self {
        Self {
            target,
            from,
            env: env.clone(),
            options: NodeResolveOptions::default_for(env),
        }
    }

    /// Resolve the path
    /// Reference: https://nodejs.org/api/modules.html#all-together
    pub fn resolve(&self) -> NodeResolverResult<String> {
        if matches!(self.env, TargetEnv::Node) {
            if self.target.starts_with("node:") {
                return Ok(String::from(self.target));
            } else if NODE_CORE_MODULES.binary_search(&self.target).is_ok() {
                return Ok(format!("node:{}", self.target));
            }
        }

        let abs_from = self.from.canonicalize().map_err(|e| {
            NodeResolverError::IOError(
                e,
                format!(
                    "Cannot resolve from file {}. Does the file exist?",
                    self.from.to_string_lossy()
                ),
            )
        })?;

        // If X begins with './' or '/' or '../'
        if self.target.starts_with('.') || self.target.starts_with('/') {
            // a. LOAD_AS_FILE(Y + X)
            let abs_to = abs_from.with_file_name(self.target);

            let as_file = self.load_as_file(&abs_to, DEFAULT_EXTENSIONS);

            if let Some(f) = as_file {
                // TODO: make this compact
                return NodeResolverResult::Ok(f.to_string_lossy().into());
            }

            let as_directory = self.load_as_directory(&abs_to);

            if let Some(f) = as_directory {
                return NodeResolverResult::Ok(f.to_string_lossy().into());
            }
        } else {
            let maybe_from_dir = abs_from.parent();
            if let Some(from_dir) = maybe_from_dir {
                let from_dir = PathBuf::from(from_dir);
                let as_node_module = self.load_node_modules(&from_dir, self.target);

                if let Some(f) = as_node_module {
                    // TODO: make this compact
                    return NodeResolverResult::Ok(f.to_string_lossy().into());
                }
            }
        }

        return Ok(format!(""));
    }

    /// Here we follow esbuild in resolving path:
    /// Node's standard:
    /// LOAD_AS_FILE(X)
    /// 1. If X is a file, load X as its file extension format. STOP
    /// 2. If X.js is a file, load X.js as JavaScript text. STOP
    /// 3. If X.json is a file, parse X.json to a JavaScript Object. STOP
    /// 4. If X.node is a file, load X.node as binary addon. STOP
    ///
    /// Esbuild's way: https://github.com/evanw/esbuild/blob/81fa2ca2e71a0518fe1e411276593ef6ea21a380/internal/resolver/resolver.go#L1388
    ///
    fn load_as_file(&self, abs_to: &PathBuf, extensions: &[Extensions]) -> Option<PathBuf> {
        if abs_to.is_file() {
            return Some(abs_to.clone().canonicalize().unwrap());
        } else {
            for extension in extensions.iter() {
                match Self::try_extension(abs_to, extension) {
                    c @ Some(_) => return c,
                    _ => {}
                };
            }

            for (rewritten_extension, try_extensions) in REWRITTEN_EXTENSIONS.iter() {
                if abs_to.ends_with(rewritten_extension.to_str()) {
                    for extension in try_extensions.iter() {
                        match Self::try_extension(abs_to, extension) {
                            c @ Some(_) => return c,
                            _ => {}
                        };
                    }
                }
            }
        }
        None
    }

    /// Node's standard:
    /// LOAD_AS_DIRECTORY(X)
    /// 1. If X/package.json is a file,
    /// a. Parse X/package.json, and look for "main" field.
    /// b. If "main" is a falsy value, GOTO 2.
    /// c. let M = X + (json main field)
    /// d. LOAD_AS_FILE(M)
    /// e. LOAD_INDEX(M)
    /// f. LOAD_INDEX(X) DEPRECATED
    /// g. THROW "not found"
    /// 2. LOAD_INDEX(X)
    ///
    /// Esbuild: https://github.com/evanw/esbuild/blob/81fa2ca2e71a0518fe1e411276593ef6ea21a380/internal/resolver/resolver.go#L1568
    ///
    fn load_as_directory(&self, abs_to: &PathBuf) -> Option<PathBuf> {
        let package_json_path = abs_to.join(PACKAGE_JSON);

        let package_json_result = Self::load_package_json(&package_json_path);

        // Node ignores invalid package.json (can't parse, fail to load, etc...)
        if let Ok(package_json) = package_json_result {
            // LOAD_AS_FILE(M)
            // LOAD_INDEX(M)

            for main_field in self.options.main_fields.iter() {
                let maybe_path = package_json.get_main_field(&main_field);
                if let Some(path) = maybe_path {
                    let target = abs_to.join(path);

                    match self.load_as_file(&target, DEFAULT_EXTENSIONS) {
                        c @ Some(_) => return c,
                        _ => {}
                    };
                }
            }
        }

        self.load_index(abs_to)
    }

    /// Node's version:
    /// LOAD_INDEX(X)
    /// 1. If X/index.js is a file, load X/index.js as JavaScript text. STOP
    /// 2. If X/index.json is a file, parse X/index.json to a JavaScript object. STOP
    /// 3. If X/index.node is a file, load X/index.node as binary addon. STOP
    /// We do it as if we are trying on './directory/index'.
    fn load_index(&self, abs_to: &PathBuf) -> Option<PathBuf> {
        let with_index = abs_to.join("index");
        return self.load_as_file(&with_index, DEFAULT_EXTENSIONS);
    }

    fn load_package_json(p: &PathBuf) -> NodeResolverResult<PackageJSON> {
        let content = fs::read_to_string(p);

        match content {
            Ok(c) => {
                let package_json_result: Result<PackageJSON, serde_json::Error> =
                    serde_json::from_str(c.as_str());

                package_json_result.map_err(|e| NodeResolverError::InvalidPackageJSON(e))
            }
            Err(e) => Err(NodeResolverError::IOError(
                e,
                format!("Can't read package.json"),
            )),
        }
    }

    /// Node's standard
    /// LOAD_NODE_MODULES(X, START)
    /// 1. let DIRS = NODE_MODULES_PATHS(START)
    /// 2. for each DIR in DIRS:
    /// a. LOAD_PACKAGE_EXPORTS(X, DIR)
    /// b. LOAD_AS_FILE(DIR/X)
    /// c. LOAD_AS_DIRECTORY(DIR/X)
    fn load_node_modules(&self, from_dir: &PathBuf, name: &str) -> Option<PathBuf> {
        let mut maybe_cur_dir = Some(from_dir.clone());

        while maybe_cur_dir.is_some() {
            let cur_dir = maybe_cur_dir.unwrap();

            // TODO: LOAD_PACKAGE_EXPORTS

            let module_base = cur_dir.join("node_modules").join(name);

            match self.load_as_file(&module_base, DEFAULT_EXTENSIONS) {
                c @ Some(_) => return c,
                _ => {}
            };

            match self.load_as_directory(&module_base) {
                c @ Some(_) => return c,
                _ => {}
            };

            maybe_cur_dir = cur_dir.parent().map(|c| PathBuf::from(c));
        }

        None
    }

    fn try_extension(abs_to: &PathBuf, extension: &Extensions) -> Option<PathBuf> {
        let extension_str = extension.to_str();
        let with_extension = abs_to.with_extension(extension_str);

        if with_extension.exists() {
            return Some(PathBuf::from(with_extension.canonicalize().unwrap()));
        }
        None
    }
}
