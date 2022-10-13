#![doc = include_str!("../README.md")]

mod es_resolver;
mod types;
mod data;
mod utils;

#[cfg(test)]
mod tests;

pub use es_resolver::EsResolver;
pub use types::{
  TargetEnv,
  EsResolverError,
  EsResolveOptions,
};
