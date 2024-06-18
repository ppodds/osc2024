#[derive(Debug)]
pub struct PartitionEntry {
    pub first_sector_lba: u32,
    pub total_sectors: u32,
}
