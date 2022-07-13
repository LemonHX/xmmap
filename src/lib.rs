mod common_builder;

use std::slice;

// default export the common builder
pub use common_builder::*;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(unix)]
pub mod unix;
#[cfg(unix)]
pub use unix::*;

pub trait MmapRawDescriptor {
    fn raw_descriptor(&self) -> RawDescriptor;
}

#[derive(Clone, Debug, Default)]
pub struct MmapBuilder {
    // ===== common =====
    pub(crate) offset: u64,
    /// `libc::mmap` does not support zero-size mappings. POSIX defines:
    ///
    /// https://pubs.opengroup.org/onlinepubs/9699919799/functions/mmap.html
    /// > If `len` is zero, `mmap()` shall fail and no mapping shall be
    /// > established.
    pub(crate) len: usize,
    /// no access then it is read only
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) execute: bool,
    /// no discriptor means is an anonymous mapping
    pub(crate) descriptor: Option<RawDescriptor>,
    // ===== common huge page extra =====
    pub(crate) huge_page: bool,
    // ===== unix common extra =====
    /// the flag is shared but default is false
    /// so I decided to use `private` instead of `shared`
    pub(crate) private: bool,
    pub(crate) advise_normal: bool,
    pub(crate) advise_sequential: bool,
    pub(crate) advise_random: bool,
    pub(crate) advise_willneed: bool,
    pub(crate) advise_dontneed: bool,
    // ===== unix map stack extra =====
    pub(crate) map_stack: bool,
    // ===== linux extra =====
    pub(crate) map_populate: bool,
    pub(crate) huge_page_1gb: bool,
    // ===== windows extra =====
    /// write and copy_on_write are exclusive
    pub(crate) copy_on_write: bool,
}

pub trait CommonMmapMut {
    fn flush_all(&self) -> std::io::Result<()>;
    fn flush_all_non_blocking(&self) -> std::io::Result<()>;
    fn flush_range(&self, offset: usize, len: usize) -> std::io::Result<()>;
    fn flush_range_non_blocking(&self, offset: usize, len: usize) -> std::io::Result<()>;
    fn block_on_flush(&self) -> std::io::Result<()>;
    fn as_slice(&mut self) -> &mut [u8];
}

pub mod common_huge_page {
    use crate::MmapBuilder;

    pub trait CommonMmapBuilderHugePage {
        fn set_huge_page(self, huge_page: bool) -> Self;
    }

    impl CommonMmapBuilderHugePage for MmapBuilder {
        fn set_huge_page(mut self, huge_page: bool) -> Self {
            self.huge_page = huge_page;
            self
        }
    }
}

impl Mmap {
    pub fn builder() -> MmapBuilder {
        MmapBuilder::default()
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr as *mut _, self.len) }
    }
}
