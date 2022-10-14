#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use es_resolve::*;
    use tracing::Level;

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

    fn with_tracing(f: fn() -> ()) {
        let collector = tracing_subscriber::fmt()
            // filter spans/events with level TRACE or higher.
            .with_max_level(Level::DEBUG)
            // build but do not install the subscriber.
            .finish();

        tracing::subscriber::with_default(collector, || {
            tracing::debug!("test tracing starts");
            f();
            tracing::debug!("test tracing ends");
        });
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
        let r = EsResolver::new("no_package_json", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/no_package_json/index.js")
        );

        // subfile
        let r = EsResolver::new("no_package_json/jsx-runtime", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/no_package_json/jsx-runtime.js")
        );

        // subfile
        let r = EsResolver::new("no_package_json/jsx-runtime", &s, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/no_package_json/jsx-runtime.js")
        );

        // multi-parent import
        let deep_source = source("node_modules_/deep/dir1/dir2/dir3/index.js");
        let r = EsResolver::new("no_package_json", &deep_source, TargetEnv::Browser);
        assert_eq!(
            r.resolve().unwrap(),
            source_str("node_modules_/node_modules/no_package_json/index.js")
        );
    }

    #[test]
    fn exports() {
        with_tracing(|| {
            let s = source("node_modules_/import_exports.mjs");

            // package subpath == "."
            let r = EsResolver::new("exports", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports/index.mjs")
            );

            // package subpath == "./nest1/nest2"
            let r = EsResolver::new("exports/nest1/nest2", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports/nest1/nest2/index.mjs")
            );

            // package subpath == "./nest2"
            let r = EsResolver::new("exports/nest2", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports/nest1/nest2/index.mjs")
            );

            // package exports array
            let r = EsResolver::new("exports_array", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports_array/index.mjs")
            );

            // scoped package
            let r = EsResolver::new("@scoped/exports/nested", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/@scoped/exports/index.mjs")
            );
        })
    }

    #[test]
    fn exports_sugar() {
        with_tracing(|| {
            let s = source("node_modules_/import_exports.mjs");

            let r = EsResolver::new("exports_sugar_string", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports_sugar_string/index.mjs")
            );

            let r = EsResolver::new("exports_sugar_object", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports_sugar_object/index.mjs")
            );

            let r = EsResolver::new("exports_sugar_array", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports_sugar_array/c.mjs")
            );

            let r = EsResolver::new("exports_sugar_array", &s, TargetEnv::Node);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports_sugar_array/a.js")
            );
        })
    }

    #[test]
    fn exports_pattern() {
        with_tracing(|| {
            let s = source("node_modules_/import_exports.mjs");

            // package subpath == "./star/*"
            let r = EsResolver::new("exports_star/star/index", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("node_modules_/node_modules/exports_star/lib/index.mjs")
            );
        });
    }

    #[test]
    fn tspaths() {
        with_tracing(|| {
            let s = source("tspaths/constant/index.ts");

            let r = EsResolver::new("constant", &s, TargetEnv::Browser);

            assert_eq!(
                r.resolve().unwrap(),
                source_str("tspaths/constant/constant.ts")
            );
        });

        with_tracing(|| {
            let s = source("tspaths/star/pages/Login.tsx");

            let r = EsResolver::new("@components/Text", &s, TargetEnv::Browser);

            assert_eq!(
                r.resolve().unwrap(),
                source_str("tspaths/star/components/Text.tsx")
            );
        });

        // If "*": [...] is not in `paths`, typescript will match all paths with baseUrl
        with_tracing(|| {
            let s = source("tspaths/star/pages/Login.tsx");

            let r = EsResolver::new("components/Text", &s, TargetEnv::Browser);

            assert_eq!(
                r.resolve().unwrap(),
                source_str("tspaths/star/components/Text.tsx")
            );
        });

        // Match to constant via star
        with_tracing(|| {
            let s = source("tspaths/star/pages/Login.tsx");

            let r = EsResolver::new("@anything/xxx", &s, TargetEnv::Browser);

            assert_eq!(
                r.resolve().unwrap(),
                source_str("tspaths/star/pages/Login.tsx")
            );

            let r = EsResolver::new("@anything/yyy", &s, TargetEnv::Browser);

            assert_eq!(
                r.resolve().unwrap(),
                source_str("tspaths/star/pages/Login.tsx")
            );
        });

        // Match "*": ["components/*"]
        with_tracing(|| {
            let s = source("tspaths/star/pages/Login.tsx");

            let r = EsResolver::new("@components/Text", &s, TargetEnv::Browser);

            assert_eq!(
                r.resolve().unwrap(),
                source_str("tspaths/star/components/Text.tsx")
            );
        });

        // Should match "@utils/high-priority/*": ["./@utils/high-priority"],
        // because it has a longer prefix
        with_tracing(|| {
            let s = source("tspaths/match-priority/index.ts");

            let r = EsResolver::new("@utils/high-priority/type", &s, TargetEnv::Browser);

            assert_eq!(
                r.resolve().unwrap(),
                source_str("tspaths/match-priority/@high-priority/type.ts")
            );
        });

        // Should handle tsconfig.json with extended JSON syntax
        with_tracing(|| {
            let s = source("tspaths/tsconfig-syntax/index.ts");

            let r = EsResolver::new("constant", &s, TargetEnv::Browser);

            assert_eq!(
                r.resolve().unwrap(),
                source_str("tspaths/tsconfig-syntax/constant.ts")
            );
        });
    }
}
