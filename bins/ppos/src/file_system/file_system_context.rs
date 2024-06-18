use alloc::rc::Weak;
use core::{any::Any, fmt::Debug};
use library::sync::mutex::Mutex;

use super::{directory_cache::DirectoryEntryOperation, file_system::FileSystemOperation};

/// The creation and reconfiguration of a superblock is governed by a file system context.
pub trait FileSystemContextOperation: Debug + Any {
    /// Get or create the mountable root and superblock.
    fn get_tree(&self) -> Result<(), &'static str>;

    fn set_root(&self, root: Weak<dyn DirectoryEntryOperation>);

    fn root(&self) -> Option<Weak<dyn DirectoryEntryOperation>>;

    fn file_system(&self) -> Weak<dyn FileSystemOperation>;
}

#[derive(Debug)]
pub struct FileSystemContext {
    root: Mutex<Option<Weak<dyn DirectoryEntryOperation>>>,
    file_system: Weak<dyn FileSystemOperation>,
}

impl FileSystemContextOperation for FileSystemContext {
    fn get_tree(&self) -> Result<(), &'static str> {
        unimplemented!()
    }

    fn set_root(&self, root: Weak<dyn DirectoryEntryOperation>) {
        *self.root.lock().unwrap() = Some(root);
    }

    fn root(&self) -> Option<Weak<dyn DirectoryEntryOperation>> {
        self.root.lock().unwrap().clone()
    }

    fn file_system(&self) -> Weak<dyn FileSystemOperation> {
        self.file_system.clone()
    }
}

impl FileSystemContext {
    pub const fn new(file_system: Weak<dyn FileSystemOperation>) -> Self {
        Self {
            root: Mutex::new(None),
            file_system,
        }
    }
}
