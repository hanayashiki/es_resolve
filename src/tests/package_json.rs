use crate::types::*;

#[test]
fn package_json() {
    {
        let pkg = r#"
            {
                "exports": "index.js"
            }
        "#;

        let package_json: PackageJSON = serde_json::from_str(pkg).unwrap();

        assert_eq!(package_json, PackageJSON {
            main: None,
            module: None,
            browser: None,
            react_native: None,
            exports: Some(Exports::String(format!("index.js"))),
        })
    }

    {
        let pkg = r#"
            {
                "exports": {
                    ".": "index.js"
                }
            }
        "#;

        let _: PackageJSON = serde_json::from_str(pkg).unwrap();
    }

    {
        let pkg = r#"
            {
                "exports": {
                    ".": {
                        "import": "index.js",
                        "require": "index.cjs"
                    }
                }
            }
        "#;

        let _: PackageJSON = serde_json::from_str(pkg).unwrap();
    }
}
