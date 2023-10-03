use crate::error::VfsError;
use crate::utils::{PollEvents, VfsDirEntry};
use crate::VfsResult;
use alloc::sync::Arc;
use downcast::{downcast_sync, AnySync};

/// Enumeration of possible methods to seek within an I/O object.
///
/// It is used by the [`Seek`] trait.
#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum SeekFrom {
    /// Sets the offset to the provided number of bytes.
    Start(u64),

    /// Sets the offset to the size of this object plus the specified number of
    /// bytes.
    ///
    /// It is possible to seek beyond the end of an object, but it's an error to
    /// seek before byte 0.
    End(i64),

    /// Sets the offset to the current position plus the specified number of
    /// bytes.
    ///
    /// It is possible to seek beyond the end of an object, but it's an error to
    /// seek before byte 0.
    Current(i64),
}
pub trait VfsFile: Send + Sync + AnySync {
    fn seek(&self, _pos: SeekFrom) -> VfsResult<u64> {
        Err(VfsError::NoSys)
    }
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
    fn readdir(&self) -> VfsResult<Option<VfsDirEntry>> {
        Err(VfsError::NoSys)
    }
    fn poll(&self, _event: PollEvents) -> VfsResult<PollEvents> {
        Err(VfsError::NoSys)
    }
    fn ioctl(&self, _cmd: u32, _arg: u64) -> VfsResult<Option<u64>> {
        Err(VfsError::NoSys)
    }
    fn mmap(&self, _offset: u64, _size: u64) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }
    fn open(&self) -> VfsResult<()> {
        Ok(())
    }
    /// Called by the close(2) system call to flush a file
    fn flush(&self) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }

    /// Called by the fsync(2) system call.
    fn fsync(&self) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }

    /// Called by the copy_file_range(2) system call.
    fn copy_file_range(
        &self,
        _offset: u64,
        _other_file: Arc<dyn VfsFile>,
        _o_offset: u64,
        _size: usize,
        _flag: u32,
    ) -> VfsResult<usize> {
        Err(VfsError::NoSys)
    }
}

downcast_sync!(dyn VfsFile);
