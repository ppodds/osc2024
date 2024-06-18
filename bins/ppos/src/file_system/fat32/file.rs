use alloc::rc::Rc;

use crate::file_system::{
    file::{File, FileOperation},
    inode::INodeOperation,
};

use super::inode::FAT32FSINode;

#[derive(Debug)]
pub struct FAT32FSFile {
    inner: File,
}

impl FAT32FSFile {
    pub const fn new(inode: Rc<FAT32FSINode>) -> Self {
        Self {
            inner: File::new(inode),
        }
    }
}

impl FileOperation for FAT32FSFile {
    fn write(&self, buf: &[u8], len: usize) -> Result<usize, &'static str> {
        unimplemented!()
    }

    fn read(&self, buf: &mut [u8], len: usize) -> Result<usize, &'static str> {
        unimplemented!()
    }

    fn position(&self) -> usize {
        self.inner.position()
    }

    fn set_position(&self, position: usize) {
        self.inner.set_position(position)
    }

    fn inode(&self) -> Rc<dyn INodeOperation> {
        self.inner.inode()
    }

    fn close(&self) {}
}
