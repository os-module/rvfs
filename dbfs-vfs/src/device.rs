use alloc::sync::Arc;
use vfscore::inode::VfsInode;
use device_interface::{BlockDevice, DeviceBase};
use constants::AlienResult;

#[derive(Clone)]
pub struct DbfsVfsDevice {
    pub device_file: Arc<dyn VfsInode>,
}

impl DbfsVfsDevice {
    pub fn new(device: Arc<dyn VfsInode>) -> Self {
        Self {
            device_file: device,
        }
    }
}

impl DeviceBase for DbfsVfsDevice {
    fn handle_irq(&self) {
        // No-op for VFS-backed device
    }
}

impl BlockDevice for DbfsVfsDevice {
    fn read(&self, buf: &mut [u8], offset: usize) -> AlienResult<usize> {
        self.device_file.read_at(offset as u64, buf).map_err(|_| constants::LinuxErrno::EIO)
    }

    fn write(&self, buf: &[u8], offset: usize) -> AlienResult<usize> {
        self.device_file.write_at(offset as u64, buf).map_err(|_| constants::LinuxErrno::EIO)
    }

    fn size(&self) -> usize {
        self.device_file.get_attr().map(|attr| attr.st_size as usize).unwrap_or(0)
    }

    fn flush(&self) -> AlienResult<()> {
        self.device_file.flush().map_err(|_| constants::LinuxErrno::EIO)
    }
}
