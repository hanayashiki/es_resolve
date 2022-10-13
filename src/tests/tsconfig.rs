use crate::types::*;
use indexmap::indexmap;

#[test]
fn tsconfig() {
    {
        let tsconfig_str = r#"
          {
            "compilerOptions": {
              "target": "ESNext",
              "useDefineForClassFields": true,
              "lib": ["DOM", "DOM.Iterable", "ESNext"],
              "allowJs": false,
              "skipLibCheck": true,
              "esModuleInterop": true,
              "allowSyntheticDefaultImports": true,
              "experimentalDecorators": true,
              "strict": true,
              "forceConsistentCasingInFileNames": true,
              "module": "ESNext",
              "moduleResolution": "Node",
              "resolveJsonModule": true,
              "isolatedModules": true,
              "noEmit": true,
              "jsx": "react-jsx",
              "baseUrl": ".",
              "paths": {
                "@/*": ["root/*"]
              }
            },
            "include": ["*"]
          }
        "#;

        let tsconfig: TSConfig = serde_json::from_str(tsconfig_str).unwrap();

        assert_eq!(
            tsconfig,
            TSConfig {
                extends: None,
                compiler_options: TSConfigCompilerOptions {
                    base_url: Some(format!(".")),
                    paths: Some(indexmap! {
                        format!("@/*") => vec![format!("root/*")],
                    }),
                }
            }
        );
    }
}
