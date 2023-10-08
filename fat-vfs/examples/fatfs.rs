use fat_vfs::{FatFs, FatFsProvider};
use spin::Mutex;
use std::error::Error;
use vfscore::fstype::VfsFsType;
use vfscore::utils::VfsTimeSpec;

#[derive(Clone)]
struct ProviderImpl;
impl FatFsProvider for ProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let fatfs = FatFs::<_, Mutex<()>>::new(ProviderImpl);
    assert_eq!(fatfs.fs_name(), "fatfs");
    Ok(())
}
