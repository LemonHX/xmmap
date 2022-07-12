mod common_builder;

// default export the common builder
pub use common_builder::*;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::*;

pub trait Access {
    fn read(&self) -> bool;
    fn write(&self) -> bool;
    fn execute(&self) -> bool;
}

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
    pub(crate) advice_normal: bool,
    pub(crate) advice_sequential: bool,
    pub(crate) advice_random: bool,
    pub(crate) advice_willneed: bool,
    pub(crate) advice_dontneed: bool,
    pub(crate) unix_flags: Option<Vec<u32>>,
    pub(crate) unix_advices: Option<Vec<u32>>,

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
    fn as_slice(&self) -> &mut [u8];
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
