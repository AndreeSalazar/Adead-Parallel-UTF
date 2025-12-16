use std::sync::Arc;
use crate::core::store::Store;
use crate::core::index::{Index, UtfId};
use crate::core::cache::Cache;
use crate::format::puf::{PufEntry, PufHeader, MAGIC};
use xxhash_rust::xxh64::xxh64;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::mem::size_of;
use bytemuck::{bytes_of, try_from_bytes};
use memmap2::Mmap;

pub struct Resolver {
    store: Store,
    index: Index,
    cache: Option<Cache>,
}

pub enum UtfRef {
    Mmap { _mmap: Arc<Mmap>, slice: *const u8, len: usize },
    Cached(Arc<str>),
}

unsafe impl Send for UtfRef {}
unsafe impl Sync for UtfRef {}

impl std::ops::Deref for UtfRef {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        match self {
            UtfRef::Mmap { slice, len, .. } => unsafe {
                let s = std::slice::from_raw_parts(*slice, *len);
                std::str::from_utf8_unchecked(s)
            },
            UtfRef::Cached(s) => s,
        }
    }
}

impl Resolver {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let store = Store::new(path)?;
        let index = Index::new();
        let cache = Some(Cache::new(1000));
        
        let resolver = Self { store, index, cache };
        resolver.load_index()?;
        Ok(resolver)
    }

    fn load_index(&self) -> Result<()> {
        let mmap_arc = self.store.get_mmap(0)?;
        if mmap_arc.len() == 0 {
            let header = PufHeader {
                magic: *MAGIC,
                version: 1,
                entry_count: 0,
            };
            self.store.append(bytes_of(&header))?;
            return Ok(());
        }

        let mmap = &mmap_arc;
        if mmap.len() < size_of::<PufHeader>() {
             return Ok(()); 
        }

        let header_slice = &mmap[0..size_of::<PufHeader>()];
        let header: &PufHeader = try_from_bytes(header_slice).map_err(|_| anyhow!("Invalid header"))?;
        
        if &header.magic != MAGIC {
            return Err(anyhow!("Invalid magic"));
        }

        let mut pos = size_of::<PufHeader>() as u64;
        let entry_size = size_of::<PufEntry>() as u64;

        loop {
            if pos + entry_size > mmap.len() as u64 {
                break;
            }
            
            let entry_slice = &mmap[pos as usize..(pos + entry_size) as usize];
            let entry: &PufEntry = try_from_bytes(entry_slice).map_err(|_| anyhow!("Invalid entry"))?;
            
            self.index.insert(entry.hash, entry.offset, entry.length);
            
            pos += entry_size + entry.length as u64;
        }

        Ok(())
    }

    pub fn register_utf(&self, text: &str) -> Result<UtfId> {
        let hash = xxh64(text.as_bytes(), 0);
        
        if self.index.contains(hash) {
            return Ok(hash);
        }

        let entry_size = size_of::<PufEntry>() as u64;
        let length = text.len() as u32;

        let write_offset = self.store.append_with_offset_builder(|current_offset| {
            let data_offset = current_offset + entry_size;
            let entry = PufEntry {
                offset: data_offset,
                length,
                _pad: 0,
                hash,
            };
            
            let mut buf = Vec::with_capacity(entry_size as usize + length as usize);
            buf.extend_from_slice(bytes_of(&entry));
            buf.extend_from_slice(text.as_bytes());
            buf
        })?;

        let data_offset = write_offset + entry_size;
        self.index.insert(hash, data_offset, length);

        Ok(hash)
    }

    pub fn resolve_utf(&self, id: UtfId) -> Option<UtfRef> {
        if let Some(cache) = &self.cache {
            if let Some(s) = cache.get(id) {
                return Some(UtfRef::Cached(s));
            }
        }
        
        let loc = self.index.get(id)?;
        
        let required_len = loc.offset + loc.length as u64;
        let mmap = self.store.get_mmap(required_len).ok()?;
        
        let slice_ptr = unsafe { mmap.as_ptr().add(loc.offset as usize) };
        
        // Optional: populate cache
        // But we need to create a string (copy) to cache it as Arc<str>
        // "RAM solo contiene referencias temporales". 
        // "cache LRU mínima (opcional)".
        // Let's NOT populate cache automatically to respect "SSD source of truth". 
        // Only explicit caching or tiny cache.
        // For now, I won't auto-populate cache to keep it zero-copy by default.

        Some(UtfRef::Mmap {
            _mmap: mmap,
            slice: slice_ptr,
            len: loc.length as usize,
        })
    }
    
    pub fn prefetch(&self, ids: &[UtfId]) {
        // "Hint al SO para acceso paralelo"
        use rayon::prelude::*;
        
        // Estrategia 1: Madvise (si está disponible, memmap2 lo expone como .advise())
        // En Windows, esto podría mapear a PrefetchVirtualMemory o simplemente no hacer nada.
        // Pero para ser portables y asegurar el efecto, usamos el "Touch" paralelo.
        
        // Recolectar rangos de memoria a tocar
        let ranges: Vec<_> = ids.par_iter()
            .filter_map(|id| self.index.get(*id))
            .collect();

        // Estrategia 2: Parallel Touch (Universal)
        // Disparamos lecturas en hilos paralelos para forzar Page Faults concurrentes.
        ranges.par_iter().for_each(|loc| {
            if let Ok(mmap) = self.store.get_mmap(loc.offset + loc.length as u64) {
                 unsafe {
                     // Calculamos la dirección de memoria
                     let ptr = mmap.as_ptr().add(loc.offset as usize);
                     // Leemos el primer byte de cada página (4096 bytes) en el rango
                     // Volatile read para asegurar que el compilador no lo optimice.
                     let mut current = 0;
                     while current < loc.length {
                         let _ = std::ptr::read_volatile(ptr.add(current as usize));
                         current += 4096;
                     }
                     // Asegurar lectura del último byte también
                     if loc.length > 0 {
                        let _ = std::ptr::read_volatile(ptr.add((loc.length - 1) as usize));
                     }
                 }
                 
                 // Si memmap2 soporta advise en el futuro de forma estable:
                 // mmap.advise(memmap2::Advice::WillNeed, loc.offset as usize, loc.length as usize).ok();
            }
        });
    }

    pub fn stats(&self) -> ParallelStats {
        ParallelStats {
            ram_index_entries: self.index.len(),
            // ...
        }
    }
}

pub struct ParallelStats {
    pub ram_index_entries: usize,
}
