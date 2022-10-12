#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use es_resolve::*;

    fn source(s: &str) -> PathBuf {
        return PathBuf::from("tests")
            .join("fixtures")
            .join(s)
            .canonicalize()
            .unwrap();
    }

    fn source_str(s: &str) -> String {
        return source(s).to_string_lossy().into();
    }

    #[test]
    fn relative() {
        let s = source("relative/index.js");

        let r = EsResolver::new("./js.js", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/js.js"));

        let r = EsResolver::new("./js", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/js.js"));

        let r = EsResolver::new("./ts", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/ts.ts"));

        let r = EsResolver::new("./tsx", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/tsx.tsx"));

        let r = EsResolver::new("./jsx", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/jsx.jsx"));

        let r = EsResolver::new("./css", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/css.css"));

        let r = EsResolver::new("./ts.js", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/ts.ts"));

        let r = EsResolver::new("./tsx.js", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/tsx.tsx"));

        let r = EsResolver::new("./priority/target", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("relative/priority/target.tsx")
        );

        let parent_s = source("relative/parent/index.js");
        let r = EsResolver::new("../ts", &parent_s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("relative/ts.ts"));
    }

    #[test]
    fn directory() {
        let s = source("directory/index.js");

        // index.js
        let r = EsResolver::new("./pkg", &s, TargetEnv::Browser);
        assert_eq!(r.resolve().unwrap(), source_str("directory/pkg/index.js"));

        // pkg.main for node
        let r = EsResolver::new("./package_json_main", &s, TargetEnv::Node);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("directory/package_json_main/main.js")
        );

        // pkg.browser for browser
        let r = EsResolver::new("./package_json_browser", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("directory/package_json_browser/browser.js")
        );

        // implicit package/index.js
        let r = EsResolver::new("./package_json_missing_main", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("directory/package_json_missing_main/index.js")
        );
    }

    #[test]
    fn node_modules() {
        let s = source("node_modules_/index.js");

        // index.js
        let r = EsResolver::new("react", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/react/index.js")
        );

        // subfile
        let r = EsResolver::new("react/jsx-runtime", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/react/jsx-runtime.js")
        );

        // subfile
        let r = EsResolver::new("react/jsx-runtime", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/react/jsx-runtime.js")
        );

        // multi-parent import
        let deep_source = source("node_modules_/deep/dir1/dir2/dir3/index.js");
        let r = EsResolver::new("react", &deep_source, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/react/index.js")
        );
    }

    #[test]
    fn exports() {
        let s = source("node_modules_/import_exports.mjs");

        // package subpath == ""
        let r = EsResolver::new("exports", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/exports/index.mjs")
        );
    }
}
