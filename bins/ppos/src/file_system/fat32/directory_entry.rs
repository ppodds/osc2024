use alloc::{rc::Weak, string::String, vec::Vec};

use crate::file_system::{
    directory_cache::{DirectoryEntry, DirectoryEntryOperation},
    inode::INodeOperation,
    super_block::SuperBlockOperation,
};

#[derive(Debug)]
pub struct FAT32FSDirectoryEntry {
    inner: DirectoryEntry,
}

impl FAT32FSDirectoryEntry {
    pub const fn new(
        parent: Option<Weak<dyn DirectoryEntryOperation>>,
        name: String,
        inode: Weak<dyn INodeOperation>,
        super_block: Weak<dyn SuperBlockOperation>,
    ) -> Self {
        Self {
            inner: DirectoryEntry::new(parent, name, inode, super_block),
        }
    }
}

impl DirectoryEntryOperation for FAT32FSDirectoryEntry {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn parent(&self) -> Option<Weak<dyn DirectoryEntryOperation>> {
        self.inner.parent()
    }

    fn inode(&self) -> Weak<dyn INodeOperation> {
        self.inner.inode()
    }

    fn add_child(&self, child: Weak<dyn DirectoryEntryOperation>) {
        self.inner.add_child(child)
    }

    fn set_parent(&self, parent: Option<Weak<dyn DirectoryEntryOperation>>) {
        self.inner.set_parent(parent)
    }

    fn children(&self) -> Vec<Weak<dyn DirectoryEntryOperation>> {
        self.inner.children()
    }

    fn set_name(&self, name: String) {
        self.inner.set_name(name)
    }

    fn super_block(&self) -> Weak<dyn SuperBlockOperation> {
        self.inner.super_block()
    }

    fn remove_child(&self, name: &str) {
        self.inner.remove_child(name)
    }
}
