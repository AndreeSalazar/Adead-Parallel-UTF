use crate::core::resolver::Resolver;
use crate::core::index::UtfId;
use rayon::prelude::*;

pub trait ParallelResolverExt {
    fn resolve_all_parallel<F>(&self, ids: &[UtfId], callback: F)
    where F: Fn(UtfId, Option<&str>) + Sync + Send;
}

impl ParallelResolverExt for Resolver {
    fn resolve_all_parallel<F>(&self, ids: &[UtfId], callback: F)
    where F: Fn(UtfId, Option<&str>) + Sync + Send {
        ids.par_iter().for_each(|&id| {
            let result = self.resolve_utf(id);
            match result {
                Some(ref s) => callback(id, Some(s)),
                None => callback(id, None),
            }
        });
    }
}
