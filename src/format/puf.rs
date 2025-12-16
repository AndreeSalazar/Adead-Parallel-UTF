use bytemuck::{Pod, Zeroable};

pub const MAGIC: &[u8; 4] = b"PUF1";

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PufHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub entry_count: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PufEntry {
    pub offset: u64,
    pub length: u32,
    pub _pad: u32,
    pub hash: u64,
}
