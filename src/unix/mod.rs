use std::{
    os::unix::prelude::{AsRawFd, RawFd},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{MmapBuilder, MmapRawDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawDescriptor(pub RawFd);

impl<T: AsRawFd> From<&T> for RawDescriptor {
    fn from(t: &T) -> Self {
        RawDescriptor(t.as_raw_fd())
    }
}

impl MmapRawDescriptor for RawDescriptor {
    fn raw_descriptor(&self) -> RawDescriptor {
        *self
    }
}

impl AsRef<RawFd> for RawDescriptor {
    fn as_ref(&self) -> &RawFd {
        &self.0
    }
}

#[derive(Clone)]
pub struct Mmap {
    pub(crate) ptr: *mut libc::c_void,
    pub(crate) len: usize,
}

impl Mmap {
    pub fn as_mut(&self) -> MmapMut {
        MmapMut {
            ptr: self.ptr,
            len: self.len,
        }
    }
}

#[derive(Clone)]
pub struct MmapMut {
    pub(crate) ptr: *mut libc::c_void,
    pub(crate) len: usize,
}

impl MmapMut {
    pub fn as_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.len) }
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        // Any errors during unmapping/closing are ignored as the only way
        // to report them would be through panicking which is highly discouraged
        // in Drop impls, c.f. https://github.com/rust-lang/lang-team/issues/97
        unsafe {
            libc::munmap(self.ptr, self.len as _);
        }
    }
}

fn page_size() -> usize {
    static PAGE_SIZE: AtomicUsize = AtomicUsize::new(0);

    match PAGE_SIZE.load(Ordering::Relaxed) {
        0 => {
            let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };

            PAGE_SIZE.store(page_size, Ordering::Relaxed);

            page_size
        }
        page_size => page_size,
    }
}

impl MmapBuilder {
    pub fn build(self) -> std::io::Result<Mmap> {
        // TODO: large page + offset
        // private
        let flags = if self.private {
            libc::MAP_PRIVATE
        } else {
            libc::MAP_SHARED
        };
        // access
        let protection = match (self.read, self.write, self.execute) {
            (true, true, true) => {
                let protection = libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC;
                Ok(protection)
            }
            (true, true, false) => {
                let protection = libc::PROT_READ | libc::PROT_WRITE;
                Ok(protection)
            }
            (true, false, true) => {
                let protection = libc::PROT_READ | libc::PROT_EXEC;
                Ok(protection)
            }
            (true, false, false) => {
                let protection = libc::PROT_READ;
                Ok(protection)
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "invalid access",
            )),
        }?;
        let mut flags = flags;
        // populate
        #[cfg(target_os = "linux")]
        {
            if self.map_populate {
                flags |= libc::MAP_POPULATE;
            }
        }
        let (flags, raw_desc) = if let Some(fd) = self.descriptor {
            (flags, fd.0)
        } else {
            (flags | libc::MAP_ANON, -1)
        };
        // map_stack
        let flags = if self.map_stack {
            #[cfg(any(
                all(target_os = "linux", not(target_arch = "mips")),
                target_os = "freebsd",
                target_os = "android"
            ))]
            {
                flags | libc::MAP_STACK
            }
            #[cfg(not(any(
                all(target_os = "linux", not(target_arch = "mips")),
                target_os = "freebsd",
                target_os = "android"
            )))]
            {
                println!("map_stack is not supported on this platform");
                flags
            }
        } else {
            flags
        };
        // huge page
        let flags = if self.huge_page {
            #[cfg(target_os = "linux")]
            {
                if self.huge_page_1gb {
                    flags | libc::MAP_HUGE_1GB
                } else {
                    flags | libc::MAP_HUGE_2MB
                }
            }
            #[cfg(target_os = "macos")]
            {
                if self.huge_page_1gb {
                    flags | libc::SUPERPAGE_SIZE_ANY
                } else {
                    flags | libc::SUPERPAGE_SIZE_2MB
                }
            }
            #[cfg(not(target_os = "macos"))]
            #[cfg(not(target_os = "linux"))]
            {
                unimplemented!()
            }
        } else {
            flags
        };
        // advise
        let mut advise = 0;
        if self.advise_dontneed {
            advise |= libc::MADV_DONTNEED;
        }
        if self.advise_willneed {
            advise |= libc::MADV_WILLNEED;
        }
        if self.advise_dontneed && self.advise_willneed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "both dontneed and willneed are not supported",
            ));
        }
        if self.advise_normal {
            advise |= libc::MADV_NORMAL;
        }
        if self.advise_sequential {
            advise |= libc::MADV_SEQUENTIAL;
        }
        if self.advise_random {
            advise |= libc::MADV_RANDOM;
        }
        if (self.advise_normal && self.advise_sequential)
            || (self.advise_normal && self.advise_random)
            || (self.advise_sequential && self.advise_random)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "only one of normal, sequential, and random is supported",
            ));
        }
        let alignment = if self.huge_page {
            #[cfg(target_os = "linux")]
            {
                self.offset
                    % if self.huge_page_1gb {
                        1024 * 1024 * 1024
                    } else {
                        2 * 1024 * 1024
                    } as u64
            }
            #[cfg(target_os = "macos")]
            {
                self.offset % 2 * 1024 * 1024 as u64
            }
            #[cfg(not(target_os = "macos"))]
            #[cfg(not(target_os = "linux"))]
            {
                unimplemented!()
            }
        } else {
            self.offset % page_size() as u64
        };
        let aligned_offset = self.offset - alignment;
        let aligned_len = self.len + alignment as usize;
        // `libc::mmap` does not support zero-size mappings. POSIX defines:
        //
        // https://pubs.opengroup.org/onlinepubs/9699919799/functions/mmap.html
        // > If `len` is zero, `mmap()` shall fail and no mapping shall be established.
        //
        // So if we would create such a mapping, crate a one-byte mapping instead:
        let aligned_len = aligned_len.max(1);
        unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                aligned_len as libc::size_t,
                protection,
                flags,
                raw_desc,
                aligned_offset as libc::off_t,
            );
            if ptr == libc::MAP_FAILED {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(Mmap {
                    ptr,
                    len: aligned_len,
                })
            }
        }
    }
}

unsafe impl Send for MmapBuilder {}
unsafe impl Sync for MmapBuilder {}
