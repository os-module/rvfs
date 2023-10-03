use crate::dentry::VfsDentry;
use crate::error::VfsError;
use crate::fstype::VfsFsType;
use crate::utils::VfsFsStat;
use crate::VfsResult;
use alloc::sync::Arc;
use downcast::{downcast_sync, AnySync};

/// Type of superblock keying.
pub enum SuperType {
    /// Only one such superblock may exist.
    Single,
    /// As [`Super::Single`], but reconfigure if it exists.
    SingleReconf,
    /// Superblocks with different data pointers may exist.
    Keyed,
    /// Multiple independent superblocks may exist.
    Independent,
    /// Uses a block device.
    BlockDev,
}
pub trait VfsSuperBlock: Send + Sync + AnySync {
    /// Determines how superblocks for this file system type are keyed.
    /// called when VFS is writing out all dirty data associated with a superblock.
    ///
    /// The second parameter indicates whether the method should wait until the write out has been completed. Optional.
    fn sync_fs(&self, _wait: bool) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }
    /// called when the VFS needs to get filesystem statistics.
    ///
    /// This function must return a structure of type [`VfsFsStat`].
    fn stat_fs(&self) -> VfsResult<VfsFsStat>;

    /// Get the SuperBlock's type
    fn super_type(&self) -> SuperType;

    /// Get the fs type of this super block
    fn get_fs_type(&self) -> Arc<dyn VfsFsType>;

    /// Get the root dentry of this super block
    fn root_dentry(&self) -> VfsResult<Arc<dyn VfsDentry>>;
}

downcast_sync!(dyn VfsSuperBlock);
