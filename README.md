# ES Resolve

JavaScript/TypeScript module resolution in Rust

**This library is under development**

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

