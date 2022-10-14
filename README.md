# ES Resolve

JavaScript/TypeScript module resolution in Rust

# Installation

```bash
cargo add es_resolve
```

# Get Started

```rust
use std::path::{Path, PathBuf};
use es_resolve::*;

// Provide an exact path to the file from which we resolve
let source = PathBuf::from("tests/fixtures/relative/js.js");
// Construct an `es_resolve::EsResolver`, then call `resolve` to get the result.
// Also check `es_resolve::EsResolverError` for a list of errors that might occur!
let target = EsResolver::new("./ts", &source, TargetEnv::Browser).resolve().unwrap();
let expected_target_target_path = Path::new("tests/fixtures/relative/ts.ts").canonicalize().unwrap();
let expected_target = expected_target_target_path.to_string_lossy();

// We expect to get the absolute path to the resolved target module!
assert_eq!(target, expected_target);
```

# Features

## General Features

| Feature | Status | Since  | Note |
|---|---|---|---|
| Relative Module Import | 👌 | 0.1.0 | `import './App'` when there is an `./App.ts ./App.tsx ./App.js` etc.
| Non-relative Module Import | 👌 | 0.1.0 | `import '@angular/core'`. See also **Package.json Supports**.
| [TypeScript Path Mapping](https://www.typescriptlang.org/docs/handbook/module-resolution.html#path-mapping) | 👌 | 0.1.0 | `import '@/App'` when you define `baseUrl` and `paths` in a parent `tsconfig.json`.

## Package.json Supports

| Feature | Status | Since  | Note |
|---|---|---|---|
| [Subpath Exports](https://nodejs.org/api/packages.html#subpath-exports) | 👌 | 0.1.0 | `{ "exports": { "import": "./index.mjs", "require": "./index.cjs" } }` in package.json is gaining popularity.
| [Subpath Imports](https://nodejs.org/api/packages.html#subpath-imports) | 👷 |  | 

