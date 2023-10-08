use crate::fstype::{MountFlags, VfsMountPoint};
use crate::inode::VfsInode;
use crate::VfsResult;
use alloc::string::String;
use alloc::sync::Arc;
use downcast::{downcast_sync, AnySync};

pub trait VfsDentry: Send + Sync + AnySync {
    /// Return the name of this dentry
    fn name(&self) -> String;
    /// Make this dentry to  a mount point
    fn to_mount_point(
        self: Arc<Self>,
        sub_fs_root: Arc<dyn VfsDentry>,
        mount_flag: MountFlags,
    ) -> VfsResult<()>;
    /// Get the inode of this dentry
    fn inode(&self) -> VfsResult<Arc<dyn VfsInode>>;
    /// Get the mount point of this dentry
    fn mount_point(&self) -> Option<VfsMountPoint>;
    /// Remove the mount point of this dentry
    fn clear_mount_point(&self);
    /// Whether this dentry is a mount point
    fn is_mount_point(&self) -> bool {
        self.mount_point().is_some()
    }
    /// Lookup a dentry in the directory
    ///
    /// The dentry should cache it's children to speed up the lookup
    fn find(&self, path: &str) -> Option<Arc<dyn VfsDentry>>;
    /// Insert a child to this dentry and return the dentry of the child
    fn insert(
        self: Arc<Self>,
        name: &str,
        child: Arc<dyn VfsInode>,
    ) -> VfsResult<Arc<dyn VfsDentry>>;
    /// Remove a child from this dentry and return the dentry of the child
    fn remove(&self, name: &str) -> Option<Arc<dyn VfsDentry>>;
}

impl dyn VfsDentry {
    /// Insert a child to this dentry and return the dentry of the child
    ///
    /// It likes [`VfsDentry::insert`], but it will not take ownership of `self`
    pub fn i_insert(
        self: &Arc<Self>,
        name: &str,
        child: Arc<dyn VfsInode>,
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        self.clone().insert(name, child)
    }
    /// Make this dentry to  a mount point
    ///
    /// It likes [`VfsDentry::to_mount_point`], but it will not take ownership of `self`
    pub fn i_to_mount_point(
        self: &Arc<Self>,
        sub_fs_root: Arc<dyn VfsDentry>,
        mount_flag: MountFlags,
    ) -> VfsResult<()> {
        self.clone().to_mount_point(sub_fs_root, mount_flag)
    }
}

downcast_sync!(dyn VfsDentry);
