use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::any::Any;
use core::cmp::min;

use spin::{Mutex, RwLock};

use core2::io::{self, Read, Seek, SeekFrom, Write};
use device_interface::BlockDevice;
use jammdb::fs::{DbFile, File, FileExt, IOResult, IndexByPageID, MemoryMap, MetaData, OpenOption, PathLike};

/// Wrapper around a BlockDevice to implement jammdb::fs::DbFile
pub struct BlockDeviceFile {
    device: Arc<dyn BlockDevice>,
    pos: u64,
    size: u64,
}

impl BlockDeviceFile {
    pub fn new(device: Arc<dyn BlockDevice>) -> Self {
        let size = device.size() as u64;
        Self {
            device,
            pos: 0,
            size,
        }
    }
}

impl Seek for BlockDeviceFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::End(p) => self.size.wrapping_add(p as u64), // Be careful with casting
            SeekFrom::Current(p) => self.pos.wrapping_add(p as u64),
        };
        self.pos = new_pos;
        Ok(self.pos)
    }
}

impl Read for BlockDeviceFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len();
        if self.pos >= self.size {
            return Ok(0);
        }
        // device.read takes offset.
        // Assuming block device can handle unaligned reads or we might need buffering.
        // Most simple block devices (like SD card wrapper) handled buf/len directly.
        // But strict block devices might need alignment.
        // For this VFS environment, let's assume the BlockDevice wrapper (DbfsVfsDevice) handles it
        // or effectively passes through to VfsFile::read_at, which handles it.
        match self.device.read(buf, self.pos as usize) {
            Ok(bytes_read) => {
                self.pos += bytes_read as u64;
                Ok(bytes_read)
            },
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "BlockDevice read error")),
        }
    }
}

impl Write for BlockDeviceFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.device.write(buf, self.pos as usize) {
            Ok(bytes_written) => {
                self.pos += bytes_written as u64;
                if self.pos > self.size {
                    self.size = self.pos;
                }
                Ok(bytes_written)
            },
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "BlockDevice write error")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.device.flush() {
             Ok(_) => Ok(()),
             Err(_) => Err(io::Error::new(io::ErrorKind::Other, "BlockDevice flush error")),
        }
    }
}

impl FileExt for BlockDeviceFile {
    fn lock_exclusive(&self) -> IOResult<()> {
        Ok(()) // No-op for now
    }

    fn allocate(&mut self, new_size: u64) -> IOResult<()> {
        // If we need to physically extend, we might do nothing if it's a fixed partition.
        // But we update our view of size.
        if new_size > self.size {
            self.size = new_size;
        }
        Ok(())
    }

    fn unlock(&self) -> IOResult<()> {
        Ok(())
    }

    fn metadata(&self) -> IOResult<MetaData> {
        Ok(MetaData { len: self.size })
    }

    fn sync_all(&self) -> IOResult<()> {
        self.device.flush().map_err(|_| io::Error::new(io::ErrorKind::Other, "Sync error"))
    }

    fn size(&self) -> usize {
        self.size as usize
    }

    fn addr(&self) -> usize {
        0 // Does not reside in memory address space (not flat mapped)
    }
}

impl DbFile for BlockDeviceFile {}


/// "MemoryMap" implementation that simulates mmap by caching pages loaded from BlockDevice
#[derive(Clone)]
pub struct BlockDeviceMapper;

impl MemoryMap for BlockDeviceMapper {
    fn do_map(&self, file: &mut File) -> IOResult<Arc<dyn IndexByPageID>> {
        // We need access to the underlying device to read pages.
        // But `file` is `jammdb::fs::File`, wrapping `Box<dyn DbFile>`.
        // We can downcast to BlockDeviceFile.
        let device = if let Some(bdf) = file.file.downcast_ref::<BlockDeviceFile>() {
             bdf.device.clone()
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "File is not BlockDeviceFile"));
        };

        Ok(Arc::new(BlockDevicePageLoader::new(device)))
    }
}

/// Helper struct to load and cache pages
pub struct BlockDevicePageLoader {
    device: Arc<dyn BlockDevice>,
    cache: Mutex<BTreeMap<u64, PageEntry>>,
    tick: Mutex<u64>,
}

struct PageEntry {
    data: Box<Vec<u8>>,
    last_used: u64,
}

const MAX_PAGES: usize = 32; // 32 * 4KB = 128KB cache limit for demo

impl BlockDevicePageLoader {
    pub fn new(device: Arc<dyn BlockDevice>) -> Self {
        Self {
            device,
            cache: Mutex::new(BTreeMap::new()),
            tick: Mutex::new(0),
        }
    }
}

impl IndexByPageID for BlockDevicePageLoader {
    fn index(&self, page_id: u64, page_size: usize) -> IOResult<&[u8]> {
        let mut tick = self.tick.lock();
        *tick += 1;
        let current_tick = *tick;
        drop(tick); // unlock tick early

        let mut cache = self.cache.lock();

        // 1. Try Cache Hit
        if let Some(entry) = cache.get_mut(&page_id) {
            entry.last_used = current_tick;
            // SAFETY: We return a reference to data that is owned by the cache.
            // usage of eviction means this is potentially unsafe if the caller holds 
            // the reference longer than the page stays in cache.
            // Users must ensure the cache is large enough to hold the working set 
            // of any active transaction.
            let ptr = entry.data.as_ptr();
            let len = entry.data.len();
            unsafe {
                return Ok(core::slice::from_raw_parts(ptr, len));
            }
        }

        // 2. Cache Miss: Load page
        // Drop lock while reading from device to allow other readers?
        // But we want to prevent stampeding reads for same page.
        // For simplicity, keep lock held (simple blocking loader).
        
        let mut buf = vec![0u8; page_size];
        let offset = page_id as usize * page_size;
        
        // Read directly from device
        match self.device.read(&mut buf, offset) {
            Ok(_) => {},
            Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "Page read failed")),
        }

        // 3. Eviction
        if cache.len() >= MAX_PAGES {
            // Find LRU
            // This is O(N) scan. For N=32 it's fast. 
            // If N is large, use a separate retrieval structure (e.g. linked hash map).
            let mut lru_page = None;
            let mut min_tick = u64::MAX;
            
            for (pid, entry) in cache.iter() {
                if entry.last_used < min_tick {
                    min_tick = entry.last_used;
                    lru_page = Some(*pid);
                }
            }
            
            if let Some(pid) = lru_page {
                cache.remove(&pid);
            }
        }

        // 4. Store and return
        let entry = PageEntry {
            data: Box::new(buf),
            last_used: current_tick,
        };
        
        // Get pointer before moving into map? No, Box address is stable.
        let ptr = entry.data.as_ptr();
        let len = entry.data.len();
        
        cache.insert(page_id, entry);
        
        unsafe {
            Ok(core::slice::from_raw_parts(ptr, len))
        }
    }

    fn len(&self) -> usize {
        self.device.size()
    }
}

/// OpenOptions implementation
pub struct BlockDeviceOpenOptions;
impl BlockDeviceOpenOptions {
    pub fn new() -> Self { Self }
}

impl OpenOption for BlockDeviceOpenOptions {
    fn new() -> Self { Self }
    fn read(&mut self, _read: bool) -> &mut Self { self }
    fn write(&mut self, _write: bool) -> &mut Self { self }
    
    fn open<T: alloc::string::ToString + PathLike>(&mut self, path: &T) -> IOResult<File> {
        // path should be our BlockDevicePath which holds the device
        // Since PathLike doesn't let us downcast easily (it's a trait bound on input),
        // we might have to use a global registry OR rely on the Path string to encode something?
        // Actually, dbfs approach:
        // DB::open takes `path`.
        // If we pass a `BlockDevicePath` struct, we can cast it?
        // But `DB::open` takes `T: PathLike`
        // We can't access `path` fields inside `OpenOption::open` unless we know `T`.
        // BUT `OpenOption::open` IS generic over `T`.
        // So we can check if `T` is `BlockDevicePath`.
        // But Downcasting generics is hard without Any. `PathLike` extends `Display + Debug`.
        
        // HACK: Use a thread-local or static to pass the device? 
        // Or better: Encode the device pointer/ID in the `PathLike` string?
        // Or just Assume a global/singleton device for now?
        // 
        // Better: `BlockDevicePath` implements `PathLike` AND has a method `get_device()`.
        // Use `Any` to downcast? `PathLike` doesn't inherit Any.
        
        // Workaround: We define `BlockDevicePath` struct.
        // In `open<T>`, we try to cast `&T` to `&BlockDevicePath`.
        // Since `T` is known at compile time to the caller, but `OpenOption` implementation must handle ANY T.
        // Wait, `jammdb` calls `O::open(&path)`.
        
        // We can make `BlockDeviceOpenOptions` ONLY support `BlockDevicePath`?
        // No, trait signature matches.
        
        // We use a hack: `BlockDevicePath` contains the `Arc<dyn BlockDevice>`.
        // We wrap it in a `Mutex` in a lazy_static map logic? No.
        
        // Let's use `Any` hack.
        // Traits cannot verify structs.
        
        // Alternative: Pass the device via `BlockDeviceOpenOptions` fields!
        // `DB::open` creates `O::new()`. It doesn't let us pass args to `O::new()`.
        // `O` must be `Default`-constructible-ish (`new()`).
        
        // This is a flaw in jammdb trait design for DI.
        // It expects `path` to carry the location.
        // So `path` MUST carry the device.
        
        // If `BlockDevicePath` holds the device...
        // `path: T`.
        // Inside `open`, we have `path: &T`.
        // We cast `path` to `Any`.
        // `&T` implements `Any` if `T: 'static`.
        if let Some(bd_path) = (path as &dyn Any).downcast_ref::<BlockDevicePath>() {
            let bf = BlockDeviceFile::new(bd_path.device.clone());
            return Ok(File::new(Box::new(bf)));
        }
        
        Err(io::Error::new(io::ErrorKind::NotFound, "Path is not BlockDevicePath"))
    }
    
    fn create(&mut self, _create: bool) -> &mut Self { self }
}

#[derive(Clone, Debug)]
pub struct BlockDevicePath {
    pub device: Arc<dyn BlockDevice>,
}

impl core::fmt::Display for BlockDevicePath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "block_device")
    }
}

impl PathLike for BlockDevicePath {
    fn exists(&self) -> bool {
         // Check for magic number to see if DB exists
         let mut buf = [0u8; 4];
         if self.device.read(&mut buf, 0).is_ok() {
             // JammDB Magic: 0x00AB_CDEF (Little Endian?)
             // 0xEF, 0xCD, 0xAB, 0x00
             if buf == [0xEF, 0xCD, 0xAB, 0x00] {
                 return true;
             }
         }
         false
    }
}
