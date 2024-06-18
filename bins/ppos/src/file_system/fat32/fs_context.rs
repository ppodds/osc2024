use core::any::Any;

use alloc::{
    rc::{Rc, Weak},
    string::String,
    vec,
    vec::Vec,
};
use library::time::Time;
use vfs::file::{Umode, UMODE};

use crate::{
    driver::sdhost,
    file_system::{
        directory_cache::DirectoryEntryOperation,
        file_system::FileSystemOperation,
        file_system_context::{FileSystemContext, FileSystemContextOperation},
        inode::INodeOperation,
        ramfs::RamFSDirectoryEntry,
        virtual_file_system,
    },
};

use super::{
    boot_sector::BootSector, file_system::FAT32FS, inode::FAT32FSINode,
    partition_entry::PartitionEntry, super_block::FAT32FSSuperBlock,
};
use crate::file_system::super_block::SuperBlockOperation;

#[derive(Debug)]
pub struct FAT32FSContext {
    inner: FileSystemContext,
}

impl FAT32FSContext {
    pub fn new(file_system: Weak<dyn FileSystemOperation>) -> Self {
        Self {
            inner: FileSystemContext::new(file_system),
        }
    }

    fn read_disk_info(&self) -> Result<PartitionEntry, &'static str> {
        let mut buf = [0_u8; 512];
        sdhost().read_block(0, &mut buf)?;
        let first_partition_entry = &buf[446..462];
        Ok(PartitionEntry {
            first_sector_lba: u32::from_le_bytes(
                (&first_partition_entry[8..12]).try_into().unwrap(),
            ),
            total_sectors: u32::from_le_bytes((&first_partition_entry[12..16]).try_into().unwrap()),
        })
    }

    fn read_boot_sector(&self, first_sector_lba: u32) -> Result<BootSector, &'static str> {
        let mut buf = [0_u8; 512];
        sdhost().read_block(first_sector_lba, &mut buf)?;
        Ok(BootSector {
            bytes_per_sector: u16::from_le_bytes((&buf[11..13]).try_into().unwrap()),
            sectors_per_cluster: buf[13],
            reserved_sectors: u16::from_le_bytes((&buf[14..16]).try_into().unwrap()),
            num_fats: buf[16],
            sectors_per_fat: u32::from_le_bytes((&buf[36..40]).try_into().unwrap()),
            root_cluster: u32::from_le_bytes((&buf[44..48]).try_into().unwrap()),
        })
    }

    fn read_fat(
        &self,
        boot_sector: &BootSector,
        partition: &PartitionEntry,
    ) -> Result<Vec<u32>, &'static str> {
        let fat_start = (partition.first_sector_lba + boot_sector.reserved_sectors as u32) as u64;
        let fat_end = fat_start + boot_sector.sectors_per_fat as u64;
        let fat_size = boot_sector.sectors_per_fat as u64 * boot_sector.bytes_per_sector as u64;
        let mut fat = vec![0_u8; fat_size as usize];
        for i in fat_start..fat_end {
            sdhost().read_block(i as u32, &mut fat[(i - fat_start) as usize * 512..])?;
        }

        let mut fat_entries = Vec::new();
        for i in (0..fat_size).step_by(4) {
            let entry = u32::from_le_bytes((&fat[i as usize..i as usize + 4]).try_into().unwrap());
            fat_entries.push(entry);
        }

        Ok(fat_entries)
    }
}

impl FileSystemContextOperation for FAT32FSContext {
    fn get_tree(&self) -> Result<(), &'static str> {
        let partition_entry = self.read_disk_info()?;
        let boot_sector = self.read_boot_sector(partition_entry.first_sector_lba)?;
        let super_block = Rc::new(FAT32FSSuperBlock::new(
            self.inner.file_system().clone(),
            512,
            partition_entry,
            boot_sector,
        ));
        let super_block_weak = Rc::downgrade(&super_block);
        let inode = Rc::new(FAT32FSINode::new(
            Umode::new(UMODE::OWNER_READ::SET),
            0,
            0,
            Time::new(0, 0),
            Time::new(0, 0),
            Time::new(0, 0),
            0,
            Rc::downgrade(&super_block),
        ));

        let root = Rc::new(RamFSDirectoryEntry::new(
            None,
            String::from("/"),
            Rc::downgrade(&(inode.clone() as Rc<dyn INodeOperation>)),
            super_block_weak.clone(),
        ));
        let root_weak = Rc::downgrade(&(root.clone() as Rc<dyn DirectoryEntryOperation>));
        virtual_file_system().add_directory_entry(root.clone());
        super_block.add_inode(inode);
        super_block.set_root(root_weak.clone());
        self.inner.set_root(root_weak.clone());
        let file_system = (self.inner.file_system().upgrade().unwrap() as Rc<dyn Any>)
            .downcast::<FAT32FS>()
            .unwrap();
        file_system.add_super_block(super_block.clone());

        // add root directory

        Ok(())
    }

    fn set_root(&self, root: Weak<dyn DirectoryEntryOperation>) {
        self.inner.set_root(root)
    }

    fn root(&self) -> Option<Weak<dyn DirectoryEntryOperation>> {
        self.inner.root()
    }

    fn file_system(&self) -> Weak<dyn FileSystemOperation> {
        self.inner.file_system()
    }
}
