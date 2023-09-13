use crate::dentry::DentryOps;
use crate::file::FileOps;
use crate::ramfs::inode::RamFsInode;
use crate::ramfs::PageProvider;
use alloc::sync::Arc;

pub struct RamFsDentry<'a, T> {
    inode: RamFsInode<'a, T>,
}

impl<T: PageProvider> RamFsDentry<'_, T> {
    pub fn new(page_provider: &'_ T) -> Self {
        Self {
            inode: RamFsInode::new(page_provider),
        }
    }
}

impl<T: PageProvider> DentryOps for RamFsDentry<'_, T> {
    type Data = &'_ RamFsDentry<'_, T>;

    fn d_revalidate(_: Self::Data) -> bool {
        true
    }
}

pub struct RamFsFile<'a, T> {
    dentry: RamFsDentry<'a, T>,
}

impl<T: PageProvider> FileOps for RamFsFile<'_, T> {
    type Data = ();
}
