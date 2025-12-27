use alloc::sync::Arc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use core::convert::TryInto;
use spin::Mutex;

use jammdb::DB;
use dbfs2::common::{DbfsAttr, DbfsError, DbfsTimeSpec, DbfsPermission, DbfsFileType, DbfsDirEntry};
use constants::AlienResult;

use crate::device::DbfsVfsDevice;

/// The DBFS filesystem instance.
/// Holds the database connection and the block device reference.
#[derive(Clone)]
pub struct Dbfs {
    db: Arc<DB>,
    #[allow(dead_code)] // Keep reference to ensure device stays alive
    block_dev: Arc<DbfsVfsDevice>,
}

impl Dbfs {
    pub fn new(db: DB, block_dev: Arc<DbfsVfsDevice>) -> Arc<Self> {
        let fs = Arc::new(Self {
            db: Arc::new(db),
            block_dev,
        });
        
        // Ensure root inode exists
        let _ = fs.init_root();
        
        fs
    }

    fn init_root(&self) -> Result<(), DbfsError> {
        let tx = self.db.tx(true).map_err(|_| DbfsError::Io)?;
        
        // Ensure buckets exist
        let _ = tx.create_bucket("metadata").map_err(|_| DbfsError::Io)?;
        let _ = tx.create_bucket("data").map_err(|_| DbfsError::Io)?;
        let _ = tx.create_bucket("dentry").map_err(|_| DbfsError::Io)?;
        
        // Check root inode 1
        let meta_bucket = tx.get_bucket("metadata").unwrap();
        if meta_bucket.get(1u64.to_be_bytes()).is_none() {
            // Create root
             let now = DbfsTimeSpec::default(); // TODO: Use provider time
             let attr = DbfsAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                kind: DbfsFileType::Directory,
                perm: (DbfsPermission::S_IFDIR | DbfsPermission::from_bits_truncate(0o755)).bits(),
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
            };
            
            // Serialize attr
            // Ideally use serde, but simple manual packing for now
            // Just transmuting might be unsafe across endianness/padding.
            // For now, let's assume in-memory struct layout is consistent on this arch.
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    &attr as *const DbfsAttr as *const u8,
                    core::mem::size_of::<DbfsAttr>()
                )
            };
            
            meta_bucket.put(1u64.to_be_bytes(), bytes).map_err(|_| DbfsError::Io)?;
        }
        
        tx.commit().map_err(|_| DbfsError::Io)?;
        Ok(())
    }

    pub fn read(&self, ino: usize, buf: &mut [u8], offset: u64) -> Result<usize, DbfsError> {
        let tx = self.db.tx(false).map_err(|_| DbfsError::Io)?;
        let data_bucket = tx.get_bucket("data").map_err(|_| DbfsError::Io)?;
        
        // Simple implementation: Key = "f:{ino}" (One chunk per file for now, matching README limit)
        // TODO: Implement chunking
        let key = format!("f:{}", ino);
        
        if let Some(kv) = data_bucket.get(key.as_bytes()) {
            let data = kv.kv().value();
            let data_len = data.len() as u64;
            
            if offset >= data_len {
                return Ok(0);
            }
            
            let read_len = core::cmp::min(buf.len() as u64, data_len - offset) as usize;
            buf[..read_len].copy_from_slice(&data[offset as usize..offset as usize + read_len]);
            Ok(read_len)
        } else {
            Ok(0)
        }
    }

    pub fn write(&self, ino: usize, buf: &[u8], offset: u64) -> Result<usize, DbfsError> {
        let tx = self.db.tx(true).map_err(|_| DbfsError::Io)?;
        let data_bucket = tx.get_bucket("data").map_err(|_| DbfsError::Io)?;
        let meta_bucket = tx.get_bucket("metadata").map_err(|_| DbfsError::Io)?;
        
        let key = format!("f:{}", ino);
        
        // Read existing
        let mut data = Vec::new();
        if let Some(kv) = data_bucket.get(key.as_bytes()) {
            data.extend_from_slice(kv.kv().value());
        }
        
        // Resize if needed
        let end = offset as usize + buf.len();
        if end > data.len() {
            data.resize(end, 0);
        }
        
        // Write
        data[offset as usize..end].copy_from_slice(buf);
        
        // Save data
        data_bucket.put(key.as_bytes(), &data).map_err(|_| DbfsError::Io)?;
        
        // Update size in inode
        let meta_key = (ino as u64).to_be_bytes();
        if let Some(kv) = meta_bucket.get(meta_key) {
             let mut attr_bytes = Vec::from(kv.kv().value());
             if attr_bytes.len() == core::mem::size_of::<DbfsAttr>() {
                 let attr = unsafe { &mut *(attr_bytes.as_mut_ptr() as *mut DbfsAttr) };
                 if (data.len() as u64) > attr.size {
                     attr.size = data.len() as u64;
                     meta_bucket.put(meta_key, &attr_bytes).map_err(|_| DbfsError::Io)?;
                 }
             }
        }

        tx.commit().map_err(|_| DbfsError::Io)?;
        Ok(buf.len())
    }

    pub fn readdir(&self, ino: usize, entries: &mut Vec<DbfsDirEntry>) -> Result<(), DbfsError> {
        let tx = self.db.tx(false).map_err(|_| DbfsError::Io)?;
        let dentry_bucket = tx.get_bucket("dentry").map_err(|_| DbfsError::Io)?;
        
        // Dentry pattern: "d:{parent}:{child_name}" -> child_ino?
        // OR dentry bucket contains sub-buckets?
        // OR prefix scan?
        // JammDB supports cursors.
        // Let's use key prefix "d:{ino}:"
        
        let prefix = format!("d:{}:", ino);
        let cursor = dentry_bucket.cursor();
        
        for kv in cursor {
            let key = core::str::from_utf8(kv.key()).unwrap_or("");
            if key.starts_with(&prefix) {
                let name = &key[prefix.len()..];
                let child_ino = u64::from_be_bytes(kv.value().try_into().unwrap_or([0;8]));
                
                // We need type. Get attribute?
                // Optimization: Store type in dentry value?
                // Assume file for now or look it up.
                // Let's look it up.
                let meta_bucket = tx.get_bucket("metadata").unwrap();
                let child_kind = if let Some(mkv) = meta_bucket.get(child_ino.to_be_bytes()) {
                    let attr = unsafe { &*(mkv.kv().value().as_ptr() as *const DbfsAttr) };
                    attr.kind
                } else {
                    DbfsFileType::RegularFile
                };

                entries.push(DbfsDirEntry {
                    ino: child_ino as usize,
                    kind: child_kind,
                    name: name.to_string(),
                });
            }
        }
        
        // Add . and ..
        entries.insert(0, DbfsDirEntry { ino, kind: DbfsFileType::Directory, name: ".".to_string() });
         entries.insert(1, DbfsDirEntry { ino, kind: DbfsFileType::Directory, name: "..".to_string() }); // TODO: Lookup parent

        Ok(())
    }

    pub fn lookup(&self, parent: usize, name: &str) -> Result<DbfsAttr, DbfsError> {
        let tx = self.db.tx(false).map_err(|_| DbfsError::Io)?;
        let dentry_bucket = tx.get_bucket("dentry").map_err(|_| DbfsError::Io)?;
        
        let key = format!("d:{}:{}", parent, name);
        
        if let Some(kv) = dentry_bucket.get(key.as_bytes()) {
            let child_ino = u64::from_be_bytes(kv.kv().value().try_into().unwrap_or([0;8]));
            self.get_attr(child_ino as usize)
        } else {
            Err(DbfsError::NotFound)
        }
    }

    pub fn create(&self, parent: usize, name: &str, _uid: u32, _gid: u32, time: DbfsTimeSpec, perm: DbfsPermission) -> Result<DbfsAttr, DbfsError> {
         let tx = self.db.tx(true).map_err(|_| DbfsError::Io)?;
         let meta_bucket = tx.get_bucket("metadata").map_err(|_| DbfsError::Io)?;
         let dentry_bucket = tx.get_bucket("dentry").map_err(|_| DbfsError::Io)?;
         
         // Alloc Ino: simple max+1 strategy
         // TODO: Store max_ino in metadata
         let mut max_ino = 1;
         for kv in meta_bucket.cursor() {
              let ino = u64::from_be_bytes(kv.key().try_into().unwrap_or([0;8]));
              if ino > max_ino { max_ino = ino; }
         }
         let new_ino = max_ino + 1;
         
         let attr = DbfsAttr {
            ino: new_ino as usize,
            size: 0,
            blocks: 0,
            atime: time,
            mtime: time,
            ctime: time,
            crtime: time,
            kind: if perm.contains(DbfsPermission::S_IFDIR) { DbfsFileType::Directory } else { DbfsFileType::RegularFile }, // Simplified
            perm: perm.bits(),
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        
        let bytes = unsafe {
            core::slice::from_raw_parts(
                &attr as *const DbfsAttr as *const u8,
                core::mem::size_of::<DbfsAttr>()
            )
        };
        meta_bucket.put(new_ino.to_be_bytes(), bytes).map_err(|_| DbfsError::Io)?;
        
        // Link to parent
        let key = format!("d:{}:{}", parent, name);
        dentry_bucket.put(key.as_bytes(), new_ino.to_be_bytes()).map_err(|_| DbfsError::Io)?;

        tx.commit().map_err(|_| DbfsError::Io)?;
        Ok(attr)
    }

    pub fn get_attr(&self, ino: usize) -> Result<DbfsAttr, DbfsError> {
        let tx = self.db.tx(false).map_err(|_| DbfsError::Io)?;
        let meta_bucket = tx.get_bucket("metadata").map_err(|_| DbfsError::Io)?;
        
        if let Some(kv) = meta_bucket.get((ino as u64).to_be_bytes()) {
             let attr_bytes = kv.kv().value();
             if attr_bytes.len() == core::mem::size_of::<DbfsAttr>() {
                 let attr = unsafe { &*(attr_bytes.as_ptr() as *const DbfsAttr) };
                 Ok(*attr)
             } else {
                 Err(DbfsError::Io)
             }
        } else {
            Err(DbfsError::NotFound)
        }
    }

    pub fn truncate(&self, ino: usize, len: usize) -> Result<(), DbfsError> {
        let tx = self.db.tx(true).map_err(|_| DbfsError::Io)?;
        let data_bucket = tx.get_bucket("data").map_err(|_| DbfsError::Io)?;
        let meta_bucket = tx.get_bucket("metadata").map_err(|_| DbfsError::Io)?;
        
        let key = format!("f:{}", ino);
        
        let mut data = Vec::new();
        if let Some(kv) = data_bucket.get(key.as_bytes()) {
            data.extend_from_slice(kv.kv().value());
        }
        
        data.resize(len, 0);
        data_bucket.put(key.as_bytes(), &data).map_err(|_| DbfsError::Io)?;
        
         // Update size in inode
        let meta_key = (ino as u64).to_be_bytes();
        if let Some(kv) = meta_bucket.get(meta_key) {
             let mut attr_bytes = Vec::from(kv.kv().value());
             if attr_bytes.len() == core::mem::size_of::<DbfsAttr>() {
                 let attr = unsafe { &mut *(attr_bytes.as_mut_ptr() as *mut DbfsAttr) };
                 attr.size = len as u64;
                 meta_bucket.put(meta_key, &attr_bytes).map_err(|_| DbfsError::Io)?;
             }
        }
        
        tx.commit().map_err(|_| DbfsError::Io)?;
        Ok(())
    }
}
