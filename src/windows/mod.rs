#![allow(clippy::zst_offset)]

use std::{os::windows::prelude::RawHandle, slice};

use widestring::U16CString;
use winapi::{
    ctypes::c_void,
    shared::{basetsd::SIZE_T, minwindef::DWORD},
    um::{
        fileapi::FlushFileBuffers,
        handleapi::{CloseHandle, DuplicateHandle, INVALID_HANDLE_VALUE},
        memoryapi::{
            CreateFileMappingW, FlushViewOfFile, GetLargePageMinimum, MapViewOfFile,
            UnmapViewOfFile, VirtualProtect, FILE_MAP_ALL_ACCESS, FILE_MAP_COPY, FILE_MAP_EXECUTE,
            FILE_MAP_LARGE_PAGES, FILE_MAP_READ,
        },
        processthreadsapi::{GetCurrentProcess, OpenProcessToken},
        securitybaseapi::AdjustTokenPrivileges,
        sysinfoapi::GetSystemInfo,
        winbase::LookupPrivilegeValueW,
        winnt::{
            DUPLICATE_SAME_ACCESS, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_READONLY,
            PAGE_READWRITE, SEC_COMMIT, SEC_LARGE_PAGES, SE_PRIVILEGE_ENABLED,
            TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
        },
    },
};

use crate::{CommonMmapMut, MmapBuilder, MmapRawDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawDescriptor(pub RawHandle);

impl MmapRawDescriptor for RawDescriptor {
    fn raw_descriptor(&self) -> RawDescriptor {
        *self
    }
}

impl From<RawHandle> for RawDescriptor {
    fn from(handle: RawHandle) -> Self {
        RawDescriptor(handle)
    }
}

impl AsRef<RawHandle> for RawDescriptor {
    fn as_ref(&self) -> &RawHandle {
        &self.0
    }
}

fn allocation_granularity() -> usize {
    unsafe {
        let mut info = std::mem::zeroed();
        GetSystemInfo(&mut info);
        info.dwAllocationGranularity as usize
    }
}

impl MmapBuilder {
    pub fn build(self) -> std::io::Result<Mmap> {
        // create access and protection flags
        let (access, protection) = match (self.read, self.write, self.execute) {
            (true, true, true) => {
                let access = FILE_MAP_ALL_ACCESS | FILE_MAP_EXECUTE;
                let protect = PAGE_EXECUTE_READWRITE;
                Ok((access, protect))
            }
            (true, true, false) => {
                let access = FILE_MAP_ALL_ACCESS;
                let protect = PAGE_READWRITE;
                Ok((access, protect))
            }
            (true, false, true) => {
                let access = FILE_MAP_READ | FILE_MAP_EXECUTE;
                let protect = PAGE_EXECUTE_READ;
                Ok((access, protect))
            }
            (true, false, false) => {
                let access = FILE_MAP_READ;
                let protect = PAGE_READONLY;
                Ok((access, protect))
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "invalid access",
            )),
        }?;

        // check huge page support
        let (access, protection) = if self.huge_page {
            let large_page_size = unsafe { GetLargePageMinimum() };
            if large_page_size != 0 {
                println!("Huge pages are supported size {}", large_page_size);
                (access | FILE_MAP_LARGE_PAGES, protection | SEC_LARGE_PAGES)
            } else {
                (access, protection)
            }
        } else {
            (access, protection)
        };

        // check cow
        let access = if self.copy_on_write {
            access | FILE_MAP_COPY
        } else {
            access
        };

        if let Some(desc) = self.descriptor {
            let alignment = self.offset % allocation_granularity() as u64;
            let aligned_offset = self.offset - alignment as u64;
            let aligned_len = self.len.max(1) + alignment as usize;
            if aligned_len == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "invalid len",
                ));
            }

            unsafe {
                let mapping = CreateFileMappingW(
                    desc.0 as _,
                    std::ptr::null_mut(),
                    protection,
                    0,
                    0,
                    std::ptr::null(),
                );
                if mapping.is_null() {
                    return Err(std::io::Error::last_os_error());
                }

                let ptr = MapViewOfFile(
                    mapping,
                    access,
                    (aligned_offset >> 16 >> 16) as DWORD,
                    (aligned_offset & 0xffffffff) as DWORD,
                    aligned_len as SIZE_T,
                );
                CloseHandle(mapping);
                if ptr.is_null() {
                    return Err(std::io::Error::last_os_error());
                }

                let mut new_handle = 0 as RawHandle;
                let cur_proc = GetCurrentProcess();
                if DuplicateHandle(
                    cur_proc,
                    desc.0 as _,
                    cur_proc,
                    &mut new_handle as *mut _ as _,
                    0,
                    0,
                    DUPLICATE_SAME_ACCESS,
                ) != 0
                {
                    Ok(Mmap {
                        handle: Some(new_handle),
                        ptr: ptr.offset(alignment as isize),
                        len: self.len,
                    })
                } else {
                    UnmapViewOfFile(ptr);
                    Err(std::io::Error::last_os_error())
                }
            }
        }
        // then is anonymous mapping
        else {
            // Ensure a non-zero length for the underlying mapping
            let mapped_len = self.len.max(1);
            unsafe {
                // Create a mapping and view with maximum access permissions, then use
                // `VirtualProtect` to set the actual `Protection`. This way, we
                // can set more permissive protection later on.
                // Also see https://msdn.microsoft.com/en-us/library/windows/desktop/aa366537.aspx

                if self.huge_page && GetLargePageMinimum() != 0 {
                    // set privalage
                    // ** oh please look at README.md if you get an error here **
                    {
                        // get token
                        let mut token = std::ptr::null_mut();
                        OpenProcessToken(
                            GetCurrentProcess(),
                            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                            &mut token,
                        );
                        // get luid
                        let mut tp: TOKEN_PRIVILEGES = std::mem::zeroed();
                        if LookupPrivilegeValueW(
                            std::ptr::null_mut(),
                            U16CString::from_str("SeLockMemoryPrivilege")
                                .unwrap()
                                .as_mut_ptr(),
                            &mut tp.Privileges[0].Luid,
                        ) == 0
                        {
                            return Err(std::io::Error::last_os_error());
                        }
                        tp.PrivilegeCount = 1;
                        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
                        // set token
                        if AdjustTokenPrivileges(
                            token,
                            0,
                            &tp as *const _ as *mut _,
                            0,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        ) == 0
                        {
                            return Err(std::io::Error::last_os_error());
                        }
                        let err = std::io::Error::last_os_error();
                        println!("set os privilege status: {:?}", err);
                        CloseHandle(token);
                    }
                    let mapping = CreateFileMappingW(
                        INVALID_HANDLE_VALUE,
                        std::ptr::null_mut(),
                        PAGE_READWRITE | SEC_COMMIT | SEC_LARGE_PAGES,
                        (mapped_len >> 16 >> 16) as DWORD,
                        (mapped_len & 0xffffffff) as DWORD,
                        std::ptr::null(),
                    );
                    if mapping.is_null() {
                        return Err(std::io::Error::last_os_error());
                    }

                    let access = FILE_MAP_ALL_ACCESS;

                    let ptr = MapViewOfFile(mapping, access, 0, 0, mapped_len as SIZE_T);
                    CloseHandle(mapping);

                    if ptr.is_null() {
                        return Err(std::io::Error::last_os_error());
                    }

                    let mut old = 0;
                    if VirtualProtect(ptr, mapped_len as SIZE_T, protection, &mut old) != 0 {
                        Ok(Mmap {
                            handle: None,
                            ptr,
                            len: self.len,
                        })
                    } else {
                        UnmapViewOfFile(ptr);
                        Err(std::io::Error::last_os_error())
                    }
                } else {
                    let mapping = CreateFileMappingW(
                        INVALID_HANDLE_VALUE,
                        std::ptr::null_mut(),
                        PAGE_EXECUTE_READWRITE,
                        (mapped_len >> 16 >> 16) as DWORD,
                        (mapped_len & 0xffffffff) as DWORD,
                        std::ptr::null(),
                    );
                    if mapping.is_null() {
                        return Err(std::io::Error::last_os_error());
                    }
                    let access = FILE_MAP_ALL_ACCESS | FILE_MAP_EXECUTE;

                    let ptr = MapViewOfFile(mapping, access, 0, 0, mapped_len as SIZE_T);
                    CloseHandle(mapping);

                    if ptr.is_null() {
                        return Err(std::io::Error::last_os_error());
                    }

                    let mut old = 0;
                    if VirtualProtect(ptr, mapped_len as SIZE_T, protection, &mut old) != 0 {
                        Ok(Mmap {
                            handle: None,
                            ptr,
                            len: self.len,
                        })
                    } else {
                        UnmapViewOfFile(ptr);
                        Err(std::io::Error::last_os_error())
                    }
                }
            }
        }
    }
}

pub trait MmapBuilderWindowsExt {
    fn build_cow(self) -> std::io::Result<Mmap>;
}

#[derive(Clone)]
pub struct Mmap {
    handle: Option<RawHandle>,
    ptr: *mut c_void,
    len: usize,
}

impl Mmap {
    pub fn builder() -> MmapBuilder {
        MmapBuilder::default()
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr as *mut _, self.len) }
    }

    pub fn as_mut(&self) -> MmapMut {
        MmapMut {
            handle: self.handle,
            ptr: self.ptr,
            len: self.len,
        }
    }
}

#[derive(Clone, Copy)]
pub struct MmapMut {
    handle: Option<RawHandle>,
    ptr: *mut c_void,
    len: usize,
}

impl Drop for Mmap {
    fn drop(&mut self) {
        let alignment = self.ptr as usize % allocation_granularity();
        // Any errors during unmapping/closing are ignored as the only way
        // to report them would be through panicking which is highly discouraged
        // in Drop impls, c.f. https://github.com/rust-lang/lang-team/issues/97
        unsafe {
            let ptr = self.ptr.offset(-(alignment as isize));
            UnmapViewOfFile(ptr);

            if let Some(handle) = self.handle {
                CloseHandle(handle as _);
            }
        }
    }
}

impl CommonMmapMut for MmapMut {
    fn as_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr as *mut _, self.len) }
    }

    fn flush_all(&self) -> std::io::Result<()> {
        self.flush_range(0, self.len)
    }

    fn flush_all_non_blocking(&self) -> std::io::Result<()> {
        self.flush_range_non_blocking(0, self.len)
    }

    fn flush_range(&self, offset: usize, len: usize) -> std::io::Result<()> {
        self.flush_range_non_blocking(offset, len)?;
        if self.handle.is_some() {
            self.block_on_flush()?;
        }
        Ok(())
    }

    fn flush_range_non_blocking(&self, offset: usize, len: usize) -> std::io::Result<()> {
        // i know this looks too C
        if unsafe { FlushViewOfFile(self.ptr.add(offset), len as SIZE_T) } != 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    fn block_on_flush(&self) -> std::io::Result<()> {
        if let Some(handle) = self.handle {
            if unsafe { FlushFileBuffers(handle as _) } != 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "no handle"))
        }
    }
}
