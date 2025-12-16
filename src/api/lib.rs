pub use crate::core::resolver::{Resolver, UtfRef, ParallelStats};
pub use crate::core::index::UtfId;
pub use crate::core::parallel::ParallelResolverExt;
use anyhow::Result;

pub fn create_resolver(path: impl AsRef<std::path::Path>) -> Result<Resolver> {
    Resolver::new(path)
}
