use alloc::string::String;
use alloc::sync::Arc;
use vfscore::dentry::VfsDentry;
use vfscore::fstype::{MountFlags, VfsMountPoint};
use vfscore::inode::VfsInode;
use vfscore::VfsResult;

pub struct UniFsDentry;

impl VfsDentry for UniFsDentry {
    fn name(&self) -> String {
        todo!()
    }

    fn to_mount_point(
        self: Arc<Self>,
        _sub_fs_root: Arc<dyn VfsDentry>,
        _mount_flag: MountFlags,
    ) -> VfsResult<()> {
        todo!()
    }

    fn inode(&self) -> VfsResult<Arc<dyn VfsInode>> {
        todo!()
    }

    fn get_vfs_mount(&self) -> Option<VfsMountPoint> {
        todo!()
    }

    fn find(&self, _path: &str) -> Option<Arc<dyn VfsDentry>> {
        todo!()
    }

    fn insert(
        self: Arc<Self>,
        _name: &str,
        _child: Arc<dyn VfsInode>,
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        todo!()
    }

    fn remove(&self, _name: &str) -> Option<Arc<dyn VfsDentry>> {
        todo!()
    }
}
