use crate::dentry::VfsDentry;
use crate::error::VfsError;
use crate::{ VfsResult};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use downcast::{downcast_sync, AnySync};
use crate::superblock::VfsSuperBlock;
use crate::utils::{FileStat, VfsInodeMode, VfsNodeType};

pub struct InodeAttr {
    /// File mode.
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    /// File size, in bytes.
    ///
    /// For truncate
    pub size: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
}

pub trait VfsInode: Send + Sync + AnySync {
    /// Create a new node with the given `path` in the directory
    ///
    /// Return [`Ok(Arc<dyn DentryOps>)`](Ok) if it already exists.
    fn create(
        &self,
        name: &str,
        mode: VfsInodeMode,
        sb:Arc<dyn VfsSuperBlock>
    ) -> VfsResult<Arc<dyn VfsInode>>;

    /// Create a new hard link to the src dentry
    fn link(&self, _name: &str, _src: Arc<dyn VfsInode>) -> VfsResult<Arc<dyn VfsInode>> {
        Err(VfsError::NoSys)
    }
    /// Remove hard link of file `name` from dir directory
    fn unlink(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }
    /// Create a new symbolic link to the \[syn_name] file
    fn symlink(&self, _name: &str, _syn_name: &str) -> VfsResult<Arc<dyn VfsDentry>> {
        Err(VfsError::NoSys)
    }
    fn lookup(&self, name: &str, sb:Arc<dyn VfsSuperBlock>) -> VfsResult<Option<Arc<dyn VfsInode>>>;

    fn mkdir(&self, name: &str, mode: u32) -> VfsResult<Arc<dyn VfsDentry>> ;

    fn rmdir(&self, target: Arc<dyn VfsDentry>) -> VfsResult<()>;

    fn mknod(&self, _name: &str, _mode: u32, _dev: u32) -> VfsResult<Arc<dyn VfsDentry>> {
        Err(VfsError::NoSys)
    }

    fn get_link(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<String> {
        Err(VfsError::NoSys)
    }
    /// Set the attributes of the node.
    ///
    ///  This method is called by chmod(2) and related system calls.
    fn set_attr(&self, target: Arc<dyn VfsDentry>, attr: InodeAttr) -> VfsResult<()>;
    /// Get the attributes of the node.
    ///
    /// This method is called by stat(2) and related system calls.
    fn get_attr(&self, target: Arc<dyn VfsDentry>) -> VfsResult<FileStat>;
    /// Called by the VFS to list all extended attributes for a given file.
    ///
    /// This method is called by the listxattr(2) system call.
    fn list_xattr(&self, _target: Arc<dyn VfsDentry>) -> VfsResult<Vec<String>> {
        Err(VfsError::NoSys)
    }
    fn inode_type(&self) -> VfsNodeType;
}

downcast_sync!(dyn VfsInode);
