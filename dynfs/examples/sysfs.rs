use dynfs::{DynFs, DynFsKernelProvider};
use spin::Mutex;
use std::error::Error;
use std::sync::Arc;
use vfscore::fstype::VfsFsType;
use vfscore::utils::VfsTimeSpec;

#[derive(Clone)]
struct DynFsKernelProviderImpl;

impl DynFsKernelProvider for DynFsKernelProviderImpl {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let sysfs = Arc::new(DynFs::<_, Mutex<()>>::new(DynFsKernelProviderImpl, "sysfs"));
    assert_eq!(sysfs.fs_name(), "sysfs");
    Ok(())
}
