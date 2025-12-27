#![cfg(test)]

use super::*;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use vfscore::{
    inode::VfsInode,
    error::VfsError,
    utils::{VfsFileStat, VfsNodeType, VfsNodePerm, VfsTimeSpec, VfsDirEntry},
    VfsResult,
};

#[derive(Clone)]
struct TestProvider;

impl DBFSProvider for TestProvider {
    fn current_time(&self) -> VfsTimeSpec {
        VfsTimeSpec::new(0, 0)
    }
}

// Mock Block Device Inode
struct MockBlockDevice {
    data: Mutex<Vec<u8>>,
}

impl MockBlockDevice {
    fn new(size: usize) -> Arc<Self> {
        Arc::new(Self {
            data: Mutex::new(vec![0; size]),
        })
    }
}

impl VfsInode for MockBlockDevice {
    fn inode_type(&self) -> VfsNodeType {
        VfsNodeType::BlockDevice
    }

    fn get_attr(&self) -> VfsResult<VfsFileStat> {
        let mut stat = VfsFileStat::default();
        stat.st_rdev = 1; // Dummy device ID
        stat.st_size = self.data.lock().len() as u64;
        Ok(stat)
    }

    fn node_perm(&self) -> VfsNodePerm {
        VfsNodePerm::from_bits_truncate(0o666)
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
         let data = self.data.lock();
         if offset >= data.len() as u64 {
             return Ok(0);
         }
         let len = core::cmp::min(buf.len(), (data.len() as u64 - offset) as usize);
         buf[..len].copy_from_slice(&data[offset as usize..offset as usize + len]);
         Ok(len)
    }

    fn write_at(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let mut data = self.data.lock();
        let end = offset as usize + buf.len();
        if end > data.len() {
             data.resize(end, 0);
        }
        data[offset as usize..end].copy_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&self) -> VfsResult<()> { Ok(()) }
    fn fsync(&self) -> VfsResult<()> { Ok(()) }
    
    // Other methods unimplemented
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn VfsInode>> { Err(VfsError::NoSys) }
    fn create(&self, _name: &str, _ty: VfsNodeType, _perm: VfsNodePerm, _rdev: Option<u64>) -> VfsResult<Arc<dyn VfsInode>> { Err(VfsError::NoSys) }
    fn truncate(&self, _len: u64) -> VfsResult<()> { Ok(()) } // Allow truncate for file-like
    fn readlink(&self, _buf: &mut [u8]) -> VfsResult<usize> { Err(VfsError::NoSys) }
    fn symlink(&self, _name: &str, _target: &str) -> VfsResult<Arc<dyn VfsInode>> { Err(VfsError::NoSys) }
    fn unlink(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NoSys) }
    fn rmdir(&self, _name: &str) -> VfsResult<()> { Err(VfsError::NoSys) }
    fn readdir(&self, _index: usize) -> VfsResult<Option<VfsDirEntry>> { Err(VfsError::NoSys) }
}


#[test]
fn test_dbfs_persistence() {
    let provider = TestProvider;
    let fs = DBFSFs::<_, spin::Mutex<()>>::new(provider);
    
    // Create a "persistent" device in memory (1MB)
    let device = MockBlockDevice::new(1024 * 1024);
    
    // Mount 1: Format and Write
    {
        let root = fs.clone().mount(0, "/", Some(device.clone()), &[]).unwrap();
        let root_inode = root.inode().unwrap();
        
        let file = root_inode.create("persist.txt", VfsNodeType::File, VfsNodePerm::from_bits_truncate(0o666), None).unwrap();
        file.write_at(0, b"Persisted Data").unwrap();
    } // Drop mount, should sync (JammDB txs commit immediately)

    // Mount 2: Recover and Read
    // Re-use SAME device (simulating reboot with persistent disk)
    {
        // Clear fs container cache to force re-mount logic (simulate reboot)
        // Since we can't access fs_container private field easily, we create NEW fs instance
        // but it needs to share container state if we want to test caching.
        // But here we WANT to test recovery from disk, so new instance is better.
        let fs2 = DBFSFs::<_, spin::Mutex<()>>::new(TestProvider);
        let root = fs2.mount(0, "/", Some(device.clone()), &[]).unwrap();
        let root_inode = root.inode().unwrap();
        
        let found = root_inode.lookup("persist.txt").expect("File should persist");
        let mut buf = [0u8; 64];
        let len = found.read_at(0, &mut buf).unwrap();
        assert_eq!(&buf[..len], b"Persisted Data");
    }
}
