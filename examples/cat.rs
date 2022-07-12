use std::{
    io::Write,
    os::windows::prelude::{AsRawHandle, RawHandle},
};

use winapi::um::fileapi::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};
use xmmap::{CommonMmapBuilder, Mmap, RawDescriptor};

pub fn file_len(handle: RawHandle) -> std::io::Result<u64> {
    let info = unsafe {
        let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();

        let ok = GetFileInformationByHandle(handle, info.as_mut_ptr());
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }

        info.assume_init()
    };

    Ok((info.nFileSizeHigh as u64) << 32 | info.nFileSizeLow as u64)
}

fn main() -> std::io::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("supply a single path as the program argument");

    let file = std::fs::File::open(path).expect("failed to open the file");

    let mmap = Mmap::builder()
        .set_discriptor(RawDescriptor(file.as_raw_handle()))
        .set_read(true)
        .set_len(file_len(file.as_raw_handle())? as usize)
        .build()?;
    let slice = mmap.as_slice();
    std::io::stdout().write_all(slice)?;
    Ok(())
}
