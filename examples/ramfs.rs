use lvfs::dentry::DentryOps;
use lvfs::fstype::{FsType, MountFlags};
use lvfs::ramfs::RamFsDentry;
use lvfs::ramfs::{PageProvider, RamFs};
use std::alloc::{alloc, dealloc, Layout};
use std::sync::Arc;
fn main() {
    env_logger::init();
    let ramfs = RamFs::new(PageProviderImpl);

    let root = RamFs::mount(&ramfs, MountFlags::empty(), "", &[]).unwrap();
    let is_valid = RamFsDentry::d_revalidate(&root);
    println!("is_valid: {}", is_valid);
}

struct PageProviderImpl;
impl PageProvider for PageProviderImpl {
    fn alloc_pages(&self, nr_pages: usize) -> *mut u8 {
        unsafe { alloc(Layout::from_size_align(nr_pages * 4096, 4096).unwrap()) }
    }
    fn free_pages(&self, addr: *mut u8, nr_pages: usize) {
        unsafe {
            dealloc(
                addr,
                Layout::from_size_align(nr_pages * 4096, 4096).unwrap(),
            )
        }
    }
}
