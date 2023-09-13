use crate::inode::InodeOps;
use crate::ramfs::PageProvider;
use alloc::vec::Vec;

pub struct RamFsInode<'a, T> {
    /// Stores all pages of the file
    page: Vec<usize>,
    page_provider: &'a T,
}

impl<T: PageProvider> RamFsInode<'_, T> {
    pub fn new(page_provider: &'_ T) -> Self {
        Self {
            page: Vec::new(),
            page_provider,
        }
    }
}

impl<T: PageProvider> InodeOps for RamFsInode<'_, T> {
    type Data = ();
}
