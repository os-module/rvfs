mod file;
mod inode;

use crate::error::VfsError;
use crate::fstype::{FileSystemFlags, FsType, MountFlags};
use crate::ramfs::file::RamFsDentry;
use crate::superblock::{SuperBlockOps, SuperType};

pub use file::*;
pub use inode::*;

pub trait PageProvider: Send + Sync {
    fn alloc_pages(&self, nr_pages: usize) -> *mut u8;
    fn free_pages(&self, addr: *mut u8, nr_pages: usize);
}

pub struct RamFs<T> {
    page_provider: T,
}

impl<T: PageProvider> RamFs<T> {
    pub fn new(page_provider: T) -> Self {
        Self { page_provider }
    }
}

pub struct RamFsSuperBlock {}
impl SuperBlockOps for RamFsSuperBlock {
    type Data = ();
    type Context = ();
    const SUPER_TYPE: SuperType = SuperType::Single;

    fn fill_super(&self) -> Self::Context {
        todo!()
    }
}

impl<T: PageProvider> FsType for RamFs<T> {
    type Data = Self;
    type DentryType = RamFsDentry<'_, T>;
    type SuperBlockType = RamFsSuperBlock;
    type ErrorType = VfsError;
    const NAME: &'static str = "ramfs";
    const FLAGS: FileSystemFlags = FileSystemFlags::REQUIRES_DEV;

    fn mount(
        fs: &Self::Data,
        flags: MountFlags,
        dev_name: &str,
        data: &[u8],
    ) -> Self::Result<Self::DentryType> {
        let dentry = RamFsDentry::new(&fs.page_provider);
        Ok(dentry)
    }

    fn kill_sb(_: &Self::SuperBlockType) -> Self::Result<()> {
        todo!()
    }
}
