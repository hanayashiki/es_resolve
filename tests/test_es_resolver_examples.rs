mod test_util;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::test_util::with_tracing;
    use es_resolve::*;
    use tracing::Level;

    fn source(s: &str) -> PathBuf {
        return PathBuf::from("tests")
            .join("examples")
            .join(s)
            .canonicalize()
            .unwrap();
    }

    fn source_str(s: &str) -> String {
        return source(s).to_string_lossy().into();
    }

    #[test]
    fn exports_sugar() {
        with_tracing(|| {
            let s = source("emotion-styled/index.js");

            let r = EsResolver::new("@emotion/styled", &s, TargetEnv::Browser);
            assert_eq!(
                r.resolve().unwrap(),
                source_str("emotion-styled/node_modules/@emotion/styled/dist/emotion-styled.browser.esm.js")
            );
        })
    }
}