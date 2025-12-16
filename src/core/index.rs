use dashmap::DashMap;

pub type UtfId = u64;

#[derive(Debug, Clone, Copy)]
pub struct EntryLocation {
    pub offset: u64,
    pub length: u32,
}

pub struct Index {
    map: DashMap<UtfId, EntryLocation>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    pub fn insert(&self, id: UtfId, offset: u64, length: u32) {
        self.map.insert(id, EntryLocation { offset, length });
    }

    pub fn get(&self, id: UtfId) -> Option<EntryLocation> {
        self.map.get(&id).map(|v| *v)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
    
    pub fn contains(&self, id: UtfId) -> bool {
        self.map.contains_key(&id)
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}
