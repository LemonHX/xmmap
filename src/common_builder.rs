use crate::{MmapBuilder, RawDescriptor};

pub trait CommonMmapBuilder {
    // setter
    fn set_offset(self, offset: u64) -> Self;
    fn set_len(self, len: usize) -> Self;
    fn set_discriptor(self, discriptor: RawDescriptor) -> Self;
    fn set_read(self, toggle: bool) -> Self;
    fn set_write(self, toggle: bool) -> Self;
    fn set_execute(self, toggle: bool) -> Self;
    // getter
    fn offset(&self) -> u64;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn discriptor(&self) -> Option<RawDescriptor>;
    fn read(&self) -> bool;
    fn write(&self) -> bool;
    fn execute(&self) -> bool;
}

impl CommonMmapBuilder for MmapBuilder {
    fn set_offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    fn set_len(mut self, len: usize) -> Self {
        self.len = len;
        self
    }

    fn set_discriptor(mut self, discriptor: RawDescriptor) -> Self {
        self.descriptor = Some(discriptor);
        self
    }

    fn set_read(mut self, toggle: bool) -> Self {
        self.read = toggle;
        self
    }

    fn set_write(mut self, toggle: bool) -> Self {
        self.write = toggle;
        self
    }

    fn set_execute(mut self, toggle: bool) -> Self {
        self.execute = toggle;
        self
    }

    fn offset(&self) -> u64 {
        self.offset
    }

    fn len(&self) -> usize {
        self.len
    }

    fn discriptor(&self) -> Option<RawDescriptor> {
        self.descriptor
    }

    fn read(&self) -> bool {
        self.read
    }

    fn write(&self) -> bool {
        self.write
    }

    fn execute(&self) -> bool {
        self.execute
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}
