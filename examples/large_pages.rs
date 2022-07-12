use std::io::Write;

use xmmap::{common_huge_page::CommonMmapBuilderHugePage, CommonMmapBuilder, CommonMmapMut, Mmap};

fn main() -> std::io::Result<()> {
    let mmap = Mmap::builder()
        .set_read(true)
        .set_write(true)
        .set_len(1024 * 2048 as usize)
        .set_huge_page(true)
        .build()?;
    let mut mutmmap = mmap.as_mut();
    let slice = mutmmap.as_slice();
    let hello = b"hello from 2MB sized large page~!\n";
    unsafe {
        std::ptr::copy_nonoverlapping(hello.as_ptr(), slice.as_mut_ptr(), 34);
    }
    std::io::stdout().write_all(&slice[0..=34])?;
    Ok(())
}
