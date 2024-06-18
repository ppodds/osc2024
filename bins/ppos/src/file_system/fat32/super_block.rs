use alloc::rc::{Rc, Weak};

use crate::file_system::{
    directory_cache::DirectoryEntryOperation,
    file_system::FileSystemOperation,
    inode::INodeOperation,
    super_block::{SuperBlock, SuperBlockOperation},
};

use super::{boot_sector::BootSector, partition_entry::PartitionEntry};

#[derive(Debug)]
pub struct FAT32FSSuperBlock {
    inner: SuperBlock,
    block_size: usize,
    partition_entry: PartitionEntry,
    boot_sector: BootSector,
}

impl FAT32FSSuperBlock {
    pub const fn new(
        file_system: Weak<dyn FileSystemOperation>,
        block_size: usize,
        partition_entry: PartitionEntry,
        boot_sector: BootSector,
    ) -> Self {
        Self {
            inner: SuperBlock::new(file_system),
            block_size,
            partition_entry,
            boot_sector,
        }
    }
}

impl SuperBlockOperation for FAT32FSSuperBlock {
    fn add_inode(&self, inode: Rc<dyn INodeOperation>) {
        self.inner.add_inode(inode)
    }

    fn set_root(&self, root: Weak<dyn DirectoryEntryOperation>) {
        self.inner.set_root(root)
    }

    fn root(&self) -> Option<Weak<dyn DirectoryEntryOperation>> {
        self.inner.root()
    }
}
