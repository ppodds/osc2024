use alloc::{
    rc::{Rc, Weak},
    string::String,
};
use library::time::Time;
use vfs::file::Umode;

use crate::file_system::{
    directory_cache::DirectoryEntryOperation,
    file::FileOperation,
    inode::{INode, INodeOperation},
    super_block::SuperBlockOperation,
};

use super::{file::FAT32FSFile, super_block::FAT32FSSuperBlock};

#[derive(Debug)]
pub struct FAT32FSINode {
    inner: INode,
}

impl FAT32FSINode {
    pub const fn new(
        umode: Umode,
        uid: u32,
        gid: u32,
        atime: Time,
        mtime: Time,
        ctime: Time,
        size: usize,
        super_block: Weak<FAT32FSSuperBlock>,
    ) -> Self {
        Self {
            inner: INode::new(umode, uid, gid, atime, mtime, ctime, size, super_block),
        }
    }
}

impl INodeOperation for FAT32FSINode {
    fn create(
        &self,
        umode: Umode,
        name: String,
        parent: Option<Weak<dyn DirectoryEntryOperation>>,
    ) -> Rc<dyn DirectoryEntryOperation> {
        self.inner.create(umode, name, parent)
    }

    fn mkdir(
        &self,
        umode: Umode,
        name: String,
        parent: Option<Weak<dyn DirectoryEntryOperation>>,
    ) -> Rc<dyn DirectoryEntryOperation> {
        self.inner.mkdir(umode, name, parent)
    }

    fn lookup(
        &self,
        parent_directory: Rc<dyn DirectoryEntryOperation>,
        target_name: &str,
    ) -> Option<Rc<dyn DirectoryEntryOperation>> {
        self.inner.lookup(parent_directory, target_name)
    }

    fn open(&self, inode: Rc<dyn INodeOperation>) -> Rc<dyn FileOperation> {
        Rc::new(FAT32FSFile::new(
            Rc::downcast::<FAT32FSINode>(inode).unwrap(),
        ))
    }

    fn size(&self) -> usize {
        self.inner.size()
    }

    fn super_block(&self) -> Weak<dyn SuperBlockOperation> {
        self.inner.super_block()
    }
}
