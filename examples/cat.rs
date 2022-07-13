use std::io::Write;
#[cfg(windows)]
use std::os::windows::prelude::{AsRawHandle, RawHandle};

#[cfg(windows)]
use winapi::um::fileapi::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
use xmmap::{CommonMmapBuilder, Mmap, RawDescriptor};
#[cfg(windows)]
pub unsafe fn file_len<T: AsRawHandle>(handle: &T) -> std::io::Result<u64> {
    let info = {
        let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();

        if GetFileInformationByHandle(handle.as_raw_handle(), info.as_mut_ptr()) == 0 {
            return Err(std::io::Error::last_os_error());
        }

        info.assume_init()
    };

    Ok((info.nFileSizeHigh as u64) << 32 | info.nFileSizeLow as u64)
}

#[cfg(unix)]
use std::os::unix::prelude::AsRawFd;
#[cfg(unix)]
pub unsafe fn file_len<T: AsRawFd>(handle: &T) -> std::io::Result<u64> {
    let fsize = libc::lseek(handle.as_raw_fd(), 0, libc::SEEK_END);
    if fsize < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(fsize as u64)
}

fn main() -> std::io::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("supply a single path as the program argument");

    let file = std::fs::File::open(path).expect("failed to open the file");

    let mmap = Mmap::builder()
        .set_len(unsafe { file_len(&file) }? as usize)
        .set_discriptor(RawDescriptor::from(&file))
        .set_read(true)
        .build()?;
    let slice = mmap.as_slice();
    std::io::stdout().write_all(slice)?;
    Ok(())
}
