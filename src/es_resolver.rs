use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{data::*, types::*};
use path_clean::{clean, PathClean};
use tracing::debug;

#[derive(Debug)]
pub struct EsResolver<'a> {
    target: &'a str,
    from: &'a PathBuf,
    env: TargetEnv,
    options: EsResolveOptions,
}

impl<'a> EsResolver<'a> {
    pub fn new(target: &'a str, from: &'a PathBuf, env: TargetEnv) -> Self {
        Self {
            target,
            from,
            env: env.clone(),
            options: EsResolveOptions::default_for(env),
        }
    }

    fn ok_with(path: PathBuf) -> EsResolverResult<String> {
        return EsResolverResult::Ok(path.clean().to_string_lossy().into());
    }

    /// Resolve the path
    /// Reference: https://nodejs.org/api/modules.html#all-together
    #[tracing::instrument(skip(self))]
    pub fn resolve(&self) -> EsResolverResult<String> {
        if matches!(self.env, TargetEnv::Node) {
            if self.target.starts_with("node:") {
                return Ok(String::from(self.target));
            } else if NODE_CORE_MODULES.binary_search(&self.target).is_ok() {
                return Ok(format!("node:{}", self.target));
            }
        }

        let abs_from = self.from.canonicalize().map_err(|e| {
            EsResolverError::IOError(
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
                return Self::ok_with(f);
            }

            let as_directory = self.load_as_directory(&abs_to);

            if let Some(f) = as_directory {
                return EsResolverResult::Ok(f.to_string_lossy().into());
            }
        } else {
            let maybe_from_dir = abs_from.parent();
            if let Some(from_dir) = maybe_from_dir {
                let from_dir = PathBuf::from(from_dir);
                let as_node_module = self.load_node_modules(&from_dir, self.target);

                if let Ok(Some(f)) = as_node_module {
                    // TODO: make this compact
                    return Self::ok_with(f);
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
    #[tracing::instrument(skip(self))]
    fn load_as_file(&self, abs_to: &PathBuf, extensions: &[Extensions]) -> Option<PathBuf> {
        if abs_to.is_file() {
            return Some(abs_to.clone());
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
    #[tracing::instrument(skip(self))]
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

    fn load_package_json(p: &PathBuf) -> EsResolverResult<PackageJSON> {
        let content = fs::read_to_string(p);

        match content {
            Ok(c) => {
                let package_json_result: Result<PackageJSON, serde_json::Error> =
                    serde_json::from_str(c.as_str());

                package_json_result.map_err(|e| EsResolverError::InvalidPackageJSON(e))
            }
            Err(e) => Err(EsResolverError::IOError(
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
    #[tracing::instrument(skip(self))]
    fn load_node_modules(
        &self,
        from_dir: &PathBuf,
        name: &str,
    ) -> EsResolverResult<Option<PathBuf>> {
        let mut maybe_cur_dir = Some(from_dir.clone());

        while maybe_cur_dir.is_some() {
            let cur_dir = maybe_cur_dir.unwrap();

            let node_modules_dir = cur_dir.join("node_modules");

            debug!(
                node_modules_dir = format!("{:?}", node_modules_dir),
                "visiting"
            );

            match self.load_package_exports(&node_modules_dir, name) {
                c @ Ok(Some(_)) => return c,
                c @ (Err(EsResolverError::InvalidModuleSpecifier(_))
                | Err(EsResolverError::IOError(_, _))) => {
                    debug!(err = format!("{:?}", c), "load_package_exports error");
                }
                c @ _ => {
                    debug!(err = format!("{:?}", c), "load_package_exports fatal error");
                    return c;
                }
            }

            let module_base = node_modules_dir.join(name);

            match self.load_as_file(&module_base, DEFAULT_EXTENSIONS) {
                c @ Some(_) => return Ok(c),
                _ => {}
            };

            match self.load_as_directory(&module_base) {
                c @ Some(_) => return Ok(c),
                _ => {}
            };

            maybe_cur_dir = cur_dir.parent().map(|c| PathBuf::from(c));
        }

        Ok(None)
    }

    fn is_conditional_exports_main_sugar(
        &self,
        exports: &Exports,
        package_json_path: &PathBuf,
    ) -> EsResolverResult<bool> {
        match exports {
            Exports::String(_) | Exports::Array(_) => Ok(true),
            Exports::Object(map) => {
                let is_conditional_sugar = map.iter().all(|(s, _)| s.starts_with('.'));
                let not_conditional_sugar = map.iter().all(|(s, _)| !s.starts_with('.'));

                if is_conditional_sugar == !not_conditional_sugar {
                    Ok(is_conditional_sugar)
                } else {
                    Err(EsResolverError::InvalidExports(
                        format!(
                            "The `pkg.exports` at {} here is invalid. Some starts with '.' but some does not. ",
                            package_json_path.to_string_lossy(),
                        )
                    ))
                }
            }
        }
    }

    /// Reference:
    /// Node's Standard:
    ///     LOAD_PACKAGE_EXPORTS https://nodejs.org/api/modules.html#all-together
    ///     PACKAGE_IMPORTS_RESOLVE https://nodejs.org/api/esm.html#resolver-algorithm-specification
    /// Node's Source: resolve.js https://github.com/nodejs/node/blob/main/lib/internal/modules/esm/resolve.js
    #[tracing::instrument(skip(self))]
    fn load_package_exports(
        &self,
        node_modules_dir: &PathBuf,
        name: &str,
    ) -> EsResolverResult<Option<PathBuf>> {
        let (package_name, _package_subpath) = self.parse_package_name(name)?;
        

        let package_subpath = format!(".{}", _package_subpath);
        // '.' when _subpath is empty, './subpath' when name is like `pkg/subpath`.

        debug!(
            package_name = format!("{:?}", package_name),
            package_subpath = format!("{:?}", package_subpath),
            "matching package exports"
        );

        let package_json_path = node_modules_dir.join(package_name).join("package.json");

        let package_json_result = fs::read_to_string(&package_json_path).map_err(|e| {
            EsResolverError::IOError(
                e,
                format!("Can't read package.json at {:?}", package_json_path),
            )
        })?;

        let package_json: PackageJSON = serde_json::from_str(&package_json_result)
            .map_err(|e| EsResolverError::InvalidPackageJSON(e))?;

        debug!(
            package_json_path = format!("{:?}", package_json_path),
            "read package.json"
        );

        match package_json.exports {
            None => return Ok(None),
            Some(ref exports) => {
                if !package_subpath.contains("*") && !package_subpath.ends_with("/") {
                    let mut maybe_target = match exports {
                        c @ Exports::String(_) => Some(c),
                        c @ Exports::Object(ref o) => {
                            o.get(&package_subpath).unwrap_or(&None).as_ref()
                        }
                        c @ Exports::Array(_) => Some(c),
                    };

                    if self.is_conditional_exports_main_sugar(&exports, &package_json_path)?
                        && package_name == "."
                    {
                        maybe_target = Some(&exports);
                    }

                    // Found a target, w/o pattern matching
                    if let Some(target) = maybe_target {
                        debug!(
                            package_name = format!("{:?}", package_name),
                            package_subpath = format!("{:?}", package_subpath),
                            "get full non-pattern export match"
                        );
                        return self.resolve_package_target(
                            &package_json_path,
                            &target,
                            "",
                            false,
                            false,
                            false,
                        );
                    }
                }


            }
        }

        // TODO: Pattern matching

        Ok(Some(PathBuf::new()))
    }

    fn resolve_package_target(
        &self,
        package_json_path: &PathBuf,
        target: &Exports,
        subpath: &str,
        pattern: bool,
        internal: bool,
        is_pathmap: bool,
    ) -> EsResolverResult<Option<PathBuf>> {
        match target {
            Exports::String(target) => {
                return self.resolve_package_target_string(
                    package_json_path,
                    target,
                    subpath,
                    pattern,
                    internal,
                    is_pathmap,
                )
            }
            Exports::Object(object) => {
                for (key, maybe_target) in object.iter() {
                    if key == "default" || self.options.conditions.contains(key) {
                        if let Some(target) = maybe_target {
                            let result = self.resolve_package_target(
                                package_json_path,
                                target,
                                subpath,
                                pattern,
                                internal,
                                is_pathmap,
                            )?;

                            match result {
                                Some(_) => return Ok(result),
                                _ => continue,
                            }
                        }
                    }
                }
            }
            Exports::Array(targets) => {
                for target in targets.iter() {
                    let result = self.resolve_package_target(
                        package_json_path,
                        target,
                        subpath,
                        pattern,
                        internal,
                        is_pathmap,
                    );

                    match result {
                        Ok(Some(_)) => return result,
                        Err(EsResolverError::InvalidExports(_)) => continue,
                        _ => continue,
                    }
                }
            }
        };

        Err(EsResolverError::InvalidExports(format!("")))
    }

    fn resolve_package_target_string(
        &self,
        package_json_path: &PathBuf,
        target: &str,
        subpath: &str,
        pattern: bool,
        internal: bool,
        is_pathmap: bool,
    ) -> EsResolverResult<Option<PathBuf>> {
        // Note: Omit Path Verification

        let resolved = package_json_path.with_file_name(target);

        return Ok(Some(PathBuf::from(resolved)));
    }

    /// Returns: (package_name, package_subpath), where `package_subpath` is what comes after `package_name` after `name`
    fn parse_package_name(&self, name: &'a str) -> EsResolverResult<(&'a str, &'a str)> {
        let mut sep_index = name.find('/');
        let mut is_scope = false;

        if name.as_bytes()[0] == b'@' {
            is_scope = true;

            match sep_index {
                Some(i) => {
                    sep_index = name[i + 1..].find('/');
                }
                None => {
                    return Err(EsResolverError::InvalidModuleSpecifier(format!("{} is not a valid package name, because it is scoped without a slash. Valid scoped names are like '@babel/core'. ", name)));
                }
            };
        }

        let package_name = match sep_index {
            Some(i) => &name[0..i],
            None => &name,
        };

        if package_name.starts_with('.') {
            return Err(EsResolverError::InvalidModuleSpecifier(format!(
                "{} is not a valid package name, because it starts with a '.'.",
                name
            )));
        }

        if package_name.contains('%') || package_name.contains('\\') {
            return Err(EsResolverError::InvalidModuleSpecifier(format!(
                "{} is not a valid package name, because it contains '%' or '\\'.",
                name
            )));
        }

        Ok((package_name, &name[package_name.len()..]))
    }

    fn try_extension(abs_to: &PathBuf, extension: &Extensions) -> Option<PathBuf> {
        let extension_str = extension.to_str();
        let with_extension = abs_to.with_extension(extension_str);

        if with_extension.exists() {
            return Some(PathBuf::from(with_extension.clean()));
        }
        None
    }
}
