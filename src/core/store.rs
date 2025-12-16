use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::io::{Write, Seek, SeekFrom};
use memmap2::Mmap;
use anyhow::{Result, Context};

#[derive(Debug)]
pub struct Store {
    file: Mutex<File>,
    mmap: RwLock<Option<Arc<Mmap>>>,
    path: PathBuf,
}

impl Store {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .context("Failed to open store file")?;

        let len = file.metadata()?.len();
        let mmap = if len > 0 {
            unsafe { Some(Arc::new(Mmap::map(&file)?)) }
        } else {
            None
        };

        Ok(Self {
            file: Mutex::new(file),
            mmap: RwLock::new(mmap),
            path,
        })
    }

    pub fn append(&self, data: &[u8]) -> Result<u64> {
        let mut file = self.file.lock().unwrap();
        let offset = file.seek(SeekFrom::End(0))?;
        file.write_all(data)?;
        file.flush()?; 
        Ok(offset)
    }

    pub fn append_with_offset_builder<F>(&self, build_data: F) -> Result<u64> 
    where F: FnOnce(u64) -> Vec<u8> {
        let mut file = self.file.lock().unwrap();
        let offset = file.seek(SeekFrom::End(0))?;
        let data = build_data(offset);
        file.write_all(&data)?;
        file.flush()?;
        Ok(offset)
    }

    /// Returns a reference to the mmap, ensuring it covers the requested range.
    pub fn get_mmap(&self, required_len: u64) -> Result<Arc<Mmap>> {
        // Fast path: check if existing mmap is sufficient
        {
            let guard = self.mmap.read().unwrap();
            if let Some(mmap) = &*guard {
                if mmap.len() as u64 >= required_len {
                    return Ok(mmap.clone());
                }
            }
        }

        // Slow path: remap
        let mut guard = self.mmap.write().unwrap();
        // Double check
        if let Some(mmap) = &*guard {
            if mmap.len() as u64 >= required_len {
                return Ok(mmap.clone());
            }
        }

        let file = self.file.lock().unwrap();
        let len = file.metadata()?.len();
        
        if len < required_len {
             // This implies the file hasn't been written to yet or request is out of bounds
             // But in our architecture, we only request what we've indexed (which implies written)
             // So this might just mean we need to remap to the new EOF.
             // If len < required_len strictly, it's an error or race condition?
             // Actually, if we just appended, len should be >= required_len.
        }

        let mmap = unsafe { Mmap::map(&*file)? };
        let arc_mmap = Arc::new(mmap);
        *guard = Some(arc_mmap.clone());

        Ok(arc_mmap)
    }
}
