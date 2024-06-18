use alloc::{
    boxed::Box,
    rc::{Rc, Weak},
};

use crate::file_system::{
    file_system::{FileSystem, FileSystemOperation},
    file_system_context::FileSystemContextOperation,
    VFSMount,
};

use super::fs_context::FAT32FSContext;

#[derive(Debug)]
pub struct FAT32FS {
    inner: FileSystem,
}

impl FAT32FS {
    pub const fn new() -> Self {
        Self {
            inner: FileSystem::new(),
        }
    }
}

impl FileSystemOperation for FAT32FS {
    fn name(&self) -> &'static str {
        "FAT32FS"
    }

    fn mount(
        &self,
        file_system: Weak<dyn FileSystemOperation>,
        device_name: &str,
    ) -> Result<crate::file_system::VFSMount, &'static str> {
        let fs_context = FAT32FSContext::new(file_system);
        fs_context.get_tree()?;
        Ok(VFSMount::new(
            fs_context.root().unwrap().clone(),
            Rc::downgrade(&self.inner.get_super_block(0).unwrap()),
        ))
    }

    fn init_file_system_context(
        &self,
        file_system: Weak<dyn FileSystemOperation>,
    ) -> Result<Box<dyn FileSystemContextOperation>, &'static str> {
        Ok(Box::new(FAT32FSContext::new(file_system)))
    }

    fn add_super_block(
        &self,
        super_block: alloc::rc::Rc<dyn crate::file_system::super_block::SuperBlockOperation>,
    ) {
        self.inner.add_super_block(super_block)
    }

    fn get_super_block(
        &self,
        index: usize,
    ) -> Option<alloc::rc::Rc<dyn crate::file_system::super_block::SuperBlockOperation>> {
        self.inner.get_super_block(index)
    }
}
