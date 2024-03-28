use vfscore::utils::VfsTimeSpec;

pub mod dir;
pub mod file;
pub mod link;
pub mod special;

#[derive(Debug, Default)]
struct ExtFsInodeAttr {
    pub atime: VfsTimeSpec,
    pub mtime: VfsTimeSpec,
    pub ctime: VfsTimeSpec,
}
