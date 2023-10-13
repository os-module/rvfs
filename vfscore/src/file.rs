use crate::error::VfsError;
use crate::utils::{PollEvents, VfsDirEntry};
use crate::VfsResult;
use downcast_rs::{impl_downcast, DowncastSync};

pub trait VfsFile: Send + Sync + DowncastSync {
    fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::NoSys)
    }
    fn write_at(&self, _offset: u64, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::NoSys)
    }
    /// Read directory entries. This is called by the getdents(2) system call.
    ///
    /// For every call, this function will return an valid entry, or an error. If
    /// it read to the end of directory, it will return an empty entry.
    fn readdir(&self, _start_index: usize) -> VfsResult<Option<VfsDirEntry>> {
        Err(VfsError::NoSys)
    }
    fn poll(&self, _event: PollEvents) -> VfsResult<PollEvents> {
        Err(VfsError::NoSys)
    }
    fn ioctl(&self, _cmd: u32, _arg: u64) -> VfsResult<Option<u64>> {
        Err(VfsError::NoSys)
    }
    /// Called by the close(2) system call to flush a file
    fn flush(&self) -> VfsResult<()> {
        Ok(())
    }
    /// Called by the fsync(2) system call.
    fn fsync(&self) -> VfsResult<()> {
        Ok(())
    }
}

impl_downcast!(sync VfsFile);
