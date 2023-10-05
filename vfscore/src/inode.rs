use crate::error::VfsError;
use crate::file::VfsFile;
use crate::superblock::VfsSuperBlock;
use crate::utils::{FileStat, VfsNodePerm, VfsNodeType, VfsTimeSpec};
use crate::VfsResult;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use downcast::{downcast_sync, AnySync};

pub struct InodeAttr {
    /// File mode.
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    /// File size, in bytes.
    ///
    /// For truncate
    pub size: u64,
    pub atime: VfsTimeSpec,
    pub mtime: VfsTimeSpec,
    pub ctime: VfsTimeSpec,
}

pub trait VfsInode: AnySync + VfsFile {
    /// Get the super block of this dentry
    fn get_super_block(&self) -> VfsResult<Arc<dyn VfsSuperBlock>>;

    /// Create a new node with the given `path` in the directory
    fn create(
        &self,
        _name: &str,
        _ty: VfsNodeType,
        _perm: VfsNodePerm,
        _rdev: Option<u32>,
    ) -> VfsResult<Arc<dyn VfsInode>> {
        Err(VfsError::NoSys)
    }

    /// Create a new hard link to the src dentry
    fn link(&self, _name: &str, _src: Arc<dyn VfsInode>) -> VfsResult<Arc<dyn VfsInode>> {
        Err(VfsError::NoSys)
    }
    /// Remove hard link of file `name` from dir directory
    fn unlink(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }
    /// Create a new symbolic link to the \[syn_name] file
    fn symlink(&self, _name: &str, _sy_name: &str) -> VfsResult<Arc<dyn VfsInode>> {
        Err(VfsError::NoSys)
    }
    fn lookup(&self, _name: &str) -> VfsResult<Option<Arc<dyn VfsInode>>> {
        Err(VfsError::NoSys)
    }
    fn rmdir(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NoSys)
    }
    fn readlink(&self, _buf: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::NoSys)
    }
    /// Set the attributes of the node.
    ///
    ///  This method is called by chmod(2) and related system calls.
    fn set_attr(&self, attr: InodeAttr) -> VfsResult<()>;
    /// Get the attributes of the node.
    ///
    /// This method is called by stat(2) and related system calls.
    fn get_attr(&self) -> VfsResult<FileStat>;
    /// Called by the VFS to list all extended attributes for a given file.
    ///
    /// This method is called by the listxattr(2) system call.
    fn list_xattr(&self) -> VfsResult<Vec<String>> {
        Err(VfsError::NoSys)
    }
    fn inode_type(&self) -> VfsNodeType;
}

downcast_sync!(dyn VfsInode);
