use crate::types::from_vfs;
use alloc::sync::Arc;
use lwext4_rs::{BlockDeviceConfig, BlockDeviceInterface};
use vfscore::inode::VfsInode;
use vfscore::VfsResult;

#[derive(Clone)]
pub struct ExtDevice {
    pub device_file: Arc<dyn VfsInode>,
    pub config: BlockDeviceConfig,
}

impl ExtDevice {
    pub fn new(device: Arc<dyn VfsInode>) -> VfsResult<Self> {
        let stat = device.get_attr()?;
        let size = stat.st_size;
        let blk_size = stat.st_blksize;
        let res = BlockDeviceConfig {
            block_size: blk_size,
            block_count: size / blk_size as u64,
            part_size: size,
            part_offset: 0,
        };
        Ok(Self {
            device_file: device,
            config: res,
        })
    }
}

impl BlockDeviceInterface for ExtDevice {
    fn open(&mut self) -> lwext4_rs::Result<BlockDeviceConfig> {
        Ok(self.config.clone())
    }

    fn read_block(
        &mut self,
        buf: &mut [u8],
        block_id: u64,
        block_count: u32,
    ) -> lwext4_rs::Result<usize> {
        let blk_size = self.config.block_size as usize;
        assert_eq!(buf.len(), blk_size * block_count as usize);
        self.device_file
            .read_at(block_id * blk_size as u64, buf)
            .map_err(from_vfs)
    }

    fn write_block(
        &mut self,
        buf: &[u8],
        block_id: u64,
        block_count: u32,
    ) -> lwext4_rs::Result<usize> {
        let blk_size = self.config.block_size as usize;
        assert_eq!(buf.len(), blk_size * block_count as usize);
        self.device_file
            .write_at(block_id * blk_size as u64, buf)
            .map_err(from_vfs)
    }

    fn close(&mut self) -> lwext4_rs::Result<()> {
        self.device_file.flush().map_err(from_vfs)
    }

    fn lock(&mut self) -> lwext4_rs::Result<()> {
        Ok(())
    }
    fn unlock(&mut self) -> lwext4_rs::Result<()> {
        Ok(())
    }
}
