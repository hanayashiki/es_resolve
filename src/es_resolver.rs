use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{data::*, types::*, utils::*};
use path_clean::PathClean;
use tracing::debug;

#[derive(Debug)]
pub struct EsResolver<'a> {
    pub target: &'a str,
    pub from: &'a PathBuf,
    pub env: TargetEnv,
    pub options: EsResolveOptions,
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

    pub fn with_options(
        target: &'a str,
        from: &'a PathBuf,
        env: TargetEnv,
        options: &EsResolveOptions,
    ) -> Self {
        Self {
            target,
            from,
            env: env.clone(),
            options: options.clone(),
        }
    }

    fn ok_with(path: PathBuf) -> EsResolverResult<String> {
        return EsResolverResult::Ok(path.clean().to_string_lossy().into());
    }

    /// Resolve the path
    ///
    /// Reference: <https://nodejs.org/api/modules.html#all-together>
    #[tracing::instrument(skip(self))]
    pub fn resolve(&self) -> EsResolverResult<String> {
        return self.resolve_impl(false);
    }

    /// Resolve the path
    ///
    /// Reference: <https://nodejs.org/api/modules.html#all-together>
    #[tracing::instrument(skip(self))]
    fn resolve_impl(&self, is_tsconfig: bool) -> EsResolverResult<String> {
        debug!("resolving {:?} from {:?}", self.target, self.from);

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

            if let Some(r) = self.load_as_relative(&abs_to) {
                return r;
            }
        } else {
            if !is_tsconfig {
                let maybe_tsconfig = self.resolve_tsconfig(self.from);

                match maybe_tsconfig {
                    Ok(Some(tsconfig)) => {
                        if let (maybe_base_url, Some(paths)) = (
                            tsconfig.compiler_options.base_url,
                            tsconfig.compiler_options.paths,
                        ) {
                            if let Some(paths) = self.match_tsconfig_paths(
                                &maybe_base_url.unwrap_or(String::from(".")),
                                &paths,
                            ) {
                                for p in paths {
                                    if let Some(r) = self.load_as_relative(&PathBuf::from(p)) {
                                        return r;
                                    }
                                }
                            }
                        }
                    },
                    Ok(None) => {
                        debug!("cannot locate a tsconfig for {:?}", self.from);
                    }
                    Err(e) => {
                        debug!("fail to resolve tsconfig = {:?}. Note that tsconfig.json cannot have trailing comma as one of our caveats.", e);
                        return Err(e);
                    }
                }
            }

            debug!("trying to load {:?} as a node module", self.target);

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

        return Err(EsResolverError::ModuleNotFound(format!(
            "Cannot resolve {:?} from {:?}",
            self.target, self.from,
        )));
    }

    fn load_as_relative(&self, abs_to: &PathBuf) -> Option<EsResolverResult<String>> {
        let as_file = self.load_as_file(&abs_to, &self.options.extensions);

        if let Some(f) = as_file {
            return Some(Self::ok_with(f));
        }

        let as_directory = self.load_as_directory(&abs_to);

        if let Some(f) = as_directory {
            return Some(Self::ok_with(f));
        }

        None
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
            debug!("matched by exact path {}", abs_to.to_string_lossy());

            return Some(abs_to.clone());
        } else {
            for extension in extensions.iter() {
                match Self::try_extension(abs_to, extension) {
                    c @ Some(_) => {
                        debug!(
                            path = format!("{}", c.as_ref().unwrap().to_string_lossy()),
                            extension = format!("{:?}", extension),
                            "matched by appending extension"
                        );
                        return c;
                    }
                    _ => {}
                };
            }

            for (rewritten_extension, try_extensions) in REWRITTEN_EXTENSIONS.iter() {
                if abs_to.ends_with(rewritten_extension.to_str()) {
                    for extension in try_extensions.iter() {
                        match Self::try_extension(abs_to, extension) {
                            Some(p) => {
                                debug!(
                                    path = format!("{}", p.to_string_lossy()),
                                    extension = format!("{:?}", extension),
                                    "matched by rewritten extension"
                                );

                                return Some(p);
                            }
                            _ => {}
                        };
                    }
                }
            }
        }

        debug!(
            "cannot match {} with extensions {:?}",
            abs_to.to_string_lossy(),
            extensions
        );

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

                    match self.load_as_file(&target, &self.options.extensions) {
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
        return self.load_as_file(&with_index, &self.options.extensions);
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

            let node_modules_dir = cur_dir.join(NODE_MODULES);

            debug!("visiting {:?}", node_modules_dir);

            match self.load_package_exports(&node_modules_dir, name) {
                c @ Ok(Some(_)) => return c,
                Ok(None) => {
                    debug!("cannot load exports for package {}", name);
                }
                c @ (Err(EsResolverError::InvalidModuleSpecifier(_))
                | Err(EsResolverError::IOError(_, _))) => {
                    debug!(err = format!("{:?}", c), "load_package_exports error");
                }
                c @ _ => {
                    debug!(err = format!("{:?}", c), "load_package_exports fatal error");
                    return c;
                }
            }

            debug!(
                "fail to resolve by package exports at {:?}",
                node_modules_dir
            );

            let module_base = node_modules_dir.join(name);

            match self.load_as_file(&module_base, &self.options.extensions) {
                c @ Some(_) => return Ok(c),
                _ => {}
            };

            match self.load_as_directory(&module_base) {
                c @ Some(_) => return Ok(c),
                _ => {}
            };

            debug!("fail to resolve from {:?}", node_modules_dir);

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
                let is_conditional_sugar = map.iter().all(|(s, _)| !s.starts_with('.'));
                let any_conditional = map.iter().any(|(s, _)| !s.starts_with('.'));

                if is_conditional_sugar == any_conditional {
                    Ok(is_conditional_sugar)
                } else {
                    Err(EsResolverError::InvalidExports(
                        format!(
                            "The `pkg.exports` at {} here is invalid. Some keys starts with '.' but some does not. ",
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
                format!(
                    "Can't read package.json at {}",
                    package_json_path.to_string_lossy()
                ),
            )
        })?;

        let package_json: PackageJSON = serde_json::from_str(&package_json_result)
            .map_err(|e| EsResolverError::InvalidPackageJSON(e))?;

        debug!(
            package_json_path = format!("{:?}", package_json_path),
            "read package.json"
        );

        match package_json.exports {
            None => {
                debug!(
                    package_json_path = format!("{:?}", package_json_path),
                    "package.json doesn't contain an `exports` field. stop matching package exports. "
                );
                return Ok(None);
            }
            Some(ref exports) => {
                debug!(
                    package_json_path = format!("{:?}", package_json_path),
                    "package.exports is an object"
                );

                if !package_subpath.contains("*") && !package_subpath.ends_with("/") {
                    let mut maybe_target = match exports {
                        c @ Exports::String(_) => Some(c),
                        _c @ Exports::Object(ref o) => {
                            o.get(&package_subpath).unwrap_or(&None).as_ref()
                        }
                        c @ Exports::Array(_) => Some(c),
                    };

                    if self.is_conditional_exports_main_sugar(&exports, &package_json_path)?
                        && package_subpath == "."
                    {
                        debug!(
                            package_json_path = format!("{:?}", package_json_path),
                            "package.exports is 'conditional exports main sugar' and we match it"
                        );

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
                            &package_subpath,
                            "",
                            false,
                            false,
                            false,
                        );
                    }
                }

                match exports {
                    Exports::Object(ref o) => {
                        let mut best_match = format!("");

                        for (key, maybe_target) in o.iter() {
                            if let Some(_) = maybe_target {
                                if match_exports_pattern(key, &package_subpath)
                                    && pattern_key_compare(&best_match, &key) == 1
                                {
                                    best_match = key.clone();
                                }
                            }
                        }

                        let subpath = extract_exports_pattern(&best_match, &package_subpath);

                        if best_match.len() > 0 {
                            return self.resolve_package_target(
                                &package_json_path,
                                o.get(&best_match).unwrap().as_ref().unwrap(),
                                subpath,
                                &package_subpath,
                                true,
                                false,
                                false,
                            );
                        }
                    }
                    _ => {}
                };
            }
        }

        // TODO: Pattern matching

        Ok(Some(PathBuf::new()))
    }

    #[tracing::instrument(skip(self))]
    fn resolve_package_target(
        &self,
        package_json_path: &PathBuf,
        target: &Exports,
        subpath: &str, // The portion that is matched in key pattern, "" if not a pattern match
        package_subpath: &str,
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
                    package_subpath,
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
                                package_subpath,
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
                        package_subpath,
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

    #[tracing::instrument(skip(self))]
    fn resolve_package_target_string(
        &self,
        package_json_path: &PathBuf,
        target: &str,
        subpath: &str,
        package_subpath: &str,
        pattern: bool,
        internal: bool,
        is_pathmap: bool,
    ) -> EsResolverResult<Option<PathBuf>> {
        // Note: Omit path verification

        let resolved = if !pattern {
            package_json_path.with_file_name(target)
        } else {
            // Only one-star pattern is supported
            package_json_path.with_file_name(target.replacen('*', subpath, 1))
        };

        debug!(
            resolved = format!("{}", resolved.to_string_lossy()),
            pattern = pattern,
            "matched target"
        );

        return Ok(Some(resolved));
    }

    /// Returns: (package_name, package_subpath), where `package_subpath` is what comes after `package_name` after `name`
    fn parse_package_name(&self, name: &'a str) -> EsResolverResult<(&'a str, &'a str)> {
        let mut sep_index = name.find('/');

        if name.as_bytes()[0] == b'@' {
            match sep_index {
                Some(i) => {
                    sep_index = name[i + 1..].find('/').map(|j| j + i + 1);
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

    /// Reference:
    /// 1. https://github.com/dividab/tsconfig-paths/blob/master/src/tsconfig-loader.ts
    fn resolve_tsconfig(&self, from_dir: &PathBuf) -> EsResolverResult<Option<TSConfig>> {
        let mut maybe_cur_dir = Some(from_dir.clone());

        while maybe_cur_dir.is_some() {
            let cur_dir = maybe_cur_dir.unwrap();

            for tsconfig_name in TSCONFIG_NAMES {
                let tsconfig_path = cur_dir.join(tsconfig_name);
                let maybe_tsconfig = self.parse_tsconfig(&tsconfig_path)?;

                if let Some(_) = maybe_tsconfig {
                    debug!(
                        tsconfig = format!("{}", tsconfig_path.to_string_lossy()),
                        "tsconfig resolved",
                    );
                    return Ok(maybe_tsconfig);
                }
            }

            maybe_cur_dir = cur_dir.parent().map(|c| PathBuf::from(c));
        }

        debug!("tsconfig is not found");

        Ok(None)
    }

    fn parse_tsconfig(&self, path: &PathBuf) -> EsResolverResult<Option<TSConfig>> {
        // TODO: what if tsconfig has a ring?
        if let Ok(content) = fs::read_to_string(&path) {
            let stripped = json_comments::StripComments::new(content.as_bytes());
            let tsconfig_result: Result<TSConfig, _> = serde_json::from_reader(stripped);

            let mut tsconfig = tsconfig_result
                .map(|tsconfig| tsconfig)
                .map_err(|e| EsResolverError::InvalidTSConfig(e))?;

            tsconfig.compiler_options.base_url = tsconfig
                .compiler_options
                .base_url
                .map(|url| path.with_file_name(url).to_string_lossy().into());

            if let Some(ref extends) = tsconfig.extends {
                let mut tsconfig_options = self.options.clone();
                tsconfig_options.extensions = vec![Extensions::Json];

                let extended_resolver =
                    EsResolver::with_options(&extends, path, TargetEnv::Node, &self.options);

                let extended_tsconfig_path =
                    extended_resolver.resolve_impl(/* is_tsconfig */ true)?;

                let maybe_extended_tsconfig =
                    self.parse_tsconfig(&PathBuf::from(&extended_tsconfig_path))?;

                if let Some(extended_tsconfig) = maybe_extended_tsconfig {
                    tsconfig.compiler_options.base_url = tsconfig
                        .compiler_options
                        .base_url
                        .or(extended_tsconfig.compiler_options.base_url);
                    tsconfig.compiler_options.paths = tsconfig
                        .compiler_options
                        .paths
                        .or(extended_tsconfig.compiler_options.paths);

                    debug!("tsconfig extends with {}", extended_tsconfig_path);
                    return Ok(Some(tsconfig));
                } else {
                    return Err(EsResolverError::InvalidTSConfigExtend(format!(
                        "The 'extends' of {} does not resolve to a valid JSON module. Is the specifier correct?",
                        path.to_string_lossy()
                    )));
                }
            } else {
                return Ok(Some(tsconfig));
            }
        } else {
            Ok(None)
        }
    }

    #[tracing::instrument(skip(self))]
    fn match_tsconfig_paths(&self, base_url: &str, paths: &TSConfigPaths) -> Option<Vec<String>> {
        match paths.get(self.target) {
            // If it is a direct match...
            Some(paths) => {
                debug!("mapping {} to constant match {:?}", self.target, paths);

                return Some(
                    paths
                        .iter()
                        .map(|p| Path::new(base_url).join(p).to_string_lossy().into())
                        .collect(),
                );
            }
            None => {
                // Now it must be a star match...
                let mut best_key = "";

                for (key, _) in paths {
                    if match_exports_pattern(key, self.target)
                        && pattern_key_compare(best_key, key) == 1
                    {
                        best_key = key.as_str();
                    }
                }

                // TypeScript implicitly has a `*: [*]` path entry.
                if best_key.len() == 0 {
                    return Some(vec![Path::new(base_url)
                        .join(self.target)
                        .to_string_lossy()
                        .into()]);
                } else {
                    let best_key_paths = paths.get(best_key).unwrap();
                    debug!(
                        "mapping {} to pattern {:?}: {:?}",
                        self.target, best_key, best_key_paths
                    );

                    return Some(
                        best_key_paths
                            .iter()
                            .map(|p| {
                                let extracted = extract_exports_pattern(best_key, self.target);

                                let path_to_try = Path::new(base_url)
                                    .join(p.replacen('*', extracted, 1))
                                    .to_string_lossy()
                                    .into();

                                debug!("trying path {} for {}", path_to_try, self.target);
                                return path_to_try;
                            })
                            .collect(),
                    );
                }
            }
        }
    }
}
