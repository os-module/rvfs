use crate::fstype::{MountFlags, VfsMountPoint};
use crate::inode::VfsInode;
use crate::superblock::VfsSuperBlock;
use crate::VfsResult;
use alloc::string::String;
use alloc::sync::Arc;
use downcast::{downcast_sync, AnySync};
pub trait VfsDentry: Send + Sync + AnySync {
    fn name(&self) -> String;
    /// Make this dentry to  a mount point
    fn to_mount_point(
        self: Arc<Self>,
        sub_fs_root: Arc<dyn VfsDentry>,
        mount_flag: MountFlags,
    ) -> VfsResult<()>;
    /// Get the super block of this dentry
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>>;
    /// Get the inode of this dentry
    fn get_inode(&self) -> VfsResult<Arc<dyn VfsInode>>;
    /// Get the mount point of this dentry
    fn get_vfs_mount(&self) -> Option<VfsMountPoint>;
    /// Whether this dentry is a mount point
    fn is_mount_point(&self) -> bool {
        self.get_vfs_mount().is_some()
    }
    /// Lookup a dentry in the directory
    ///
    /// The dentry should cache it's children to speed up the lookup
    fn find(&self, path: &str) -> Option<Arc<dyn VfsDentry>>;
    /// Add a child to this dentry and return the dentry of the child
    fn insert(self:Arc<Self>, name:&str, child: Arc<dyn VfsInode>) -> VfsResult<Arc<dyn VfsDentry>>;
}

downcast_sync!(dyn VfsDentry);
