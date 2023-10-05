use crate::dentry::VfsDentry;
use crate::superblock::VfsSuperBlock;
use crate::VfsResult;
use alloc::sync::{Arc, Weak};
use bitflags::bitflags;
use downcast::{downcast_sync, AnySync};
bitflags! {
    pub struct FileSystemFlags:u32{
         /// The file system requires a device.
         const REQUIRES_DEV = 0x1;
        /// The options provided when mounting are in binary form.
         const BINARY_MOUNTDATA = 0x2;
        /// The file system has a subtype. It is extracted from the name and passed in as a parameter.
        const HAS_SUBTYPE = 0x4;
         /// The file system can be mounted by userns root.
        const USERNS_MOUNT = 0x8;
        /// Disables fanotify permission events.
        const DISALLOW_NOTIFY_PERM = 0x10;
        /// The file system has been updated to handle vfs idmappings.
        const ALLOW_IDMAP = 0x20;
        /// FS uses multigrain timestamps
        const MGTIME = 0x40;
        /// The file systen will handle `d_move` during `rename` internally.
        const RENAME_DOES_D_MOVE = 0x8000; //32768
    }
}

bitflags! {
    pub struct MountFlags:u32{

    }
}

pub trait VfsFsType: Send + Sync + AnySync {
    /// the method to call when a new instance of this filesystem should be mounted
    fn mount(
        self: Arc<Self>,
        flags: MountFlags,
        dev_name: &str,
        data: &[u8],
    ) -> VfsResult<Arc<dyn VfsDentry>>;
    /// unmount a filesystem
    fn kill_sb(&self, sb: Arc<dyn VfsSuperBlock>) -> VfsResult<()>;
    /// Get the flags of this filesystem
    fn fs_flag(&self) -> FileSystemFlags;
    /// Get the name of this filesystem
    fn fs_name(&self) -> &'static str;
}

downcast_sync!(dyn VfsFsType);

#[derive(Clone)]
pub struct VfsMountPoint {
    pub root: Arc<dyn VfsDentry>,
    pub mount_point: Weak<dyn VfsDentry>,
    pub mnt_flags: MountFlags,
}
