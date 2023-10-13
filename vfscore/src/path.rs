//! Utilities for path manipulation.
//!
use crate::dentry::VfsDentry;
use crate::error::VfsError;
use crate::inode::VfsInode;
use crate::utils::{VfsDirEntry, VfsNodePerm, VfsNodeType};
use crate::VfsResult;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{vec};
use core::error::Error;
use core::fmt::{write, Debug, Formatter, Write};
use log::{error};

#[derive(Clone)]
pub struct VfsPath {
    fs: Arc<dyn VfsDentry>,
    path: String,
}

impl PartialEq for VfsPath {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && Arc::ptr_eq(&self.fs, &other.fs)
    }
}

impl Eq for VfsPath {}

impl Debug for VfsPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("VfsPath({})", self.path))
    }
}

impl VfsPath {
    pub fn new(root_dentry: Arc<dyn VfsDentry>) -> Self {
        Self {
            fs: root_dentry,
            path: "".to_string(),
        }
    }
    pub fn as_str(&self) -> &str {
        &self.path
    }

    pub fn join(&self, path: impl AsRef<str>) -> VfsResult<Self> {
        self.join_internal(path.as_ref())
    }

    /// Appends a path segment to this path, returning the result
    fn join_internal(&self, path: &str) -> VfsResult<Self> {
        if path.is_empty() {
            return Ok(self.clone());
        }
        let mut new_components: Vec<&str> = vec![];
        let mut base_path = if path.starts_with('/') {
            self.root()
        } else {
            self.clone()
        };
        // Prevent paths from ending in slashes unless this is just the root directory.
        if path.len() > 1 && path.ends_with('/') {
            return Err(VfsError::Invalid);
        }
        for component in path.split('/') {
            if component == "." || component.is_empty() {
                continue;
            }
            if component == ".." {
                if !new_components.is_empty() {
                    new_components.truncate(new_components.len() - 1);
                } else {
                    base_path = base_path.parent();
                }
            } else {
                new_components.push(component);
            }
        }
        let mut path = base_path.path;
        for component in new_components {
            path += "/";
            path += component
        }
        Ok(VfsPath {
            path,
            fs: self.fs.clone(),
        })
    }
    pub fn root(&self) -> VfsPath {
        VfsPath {
            path: "".to_string(),
            fs: self.fs.clone(),
        }
    }
    pub fn is_root(&self) -> bool {
        self.path.is_empty()
    }

    pub fn open(&self) -> VfsResult<Arc<dyn VfsDentry>> {
        self.exists()?
            .map(Ok)
            .unwrap_or_else(|| Err(VfsError::NoEntry))
    }

    pub fn create_file(&self, perm: VfsNodePerm) -> VfsResult<Arc<dyn VfsDentry>> {
        self.create(VfsNodeType::File, perm, "create file")
    }

    pub fn create_dir(&self, perm: VfsNodePerm) -> VfsResult<Arc<dyn VfsDentry>> {
        self.create(VfsNodeType::Dir, perm, "create dir")
    }

    fn create(
        &self,
        ty: VfsNodeType,
        perm: VfsNodePerm,
        action: &str,
    ) -> VfsResult<Arc<dyn VfsDentry>> {
        let parent = self.get_parent(action)?;
        // resolve mount point
        let dentry = real_dentry(parent);
        let file_name = self.path.rsplit('/').next();
        if file_name.is_none() {
            return Err(VfsError::Invalid);
        }
        let file_name = file_name.unwrap();
        // first, we find in dentry cache
        let file = dentry.find(file_name);
        if file.is_none() {
            // second, we find in inode cache or disk
            let file_inode = dentry.inode()?.lookup(file_name)?;
            if file.is_some() {
                let file_inode = file_inode.unwrap();
                // if we find the inode, we insert it into dentry cache
                let _ = dentry.insert(file_name, file_inode)?;
                Err(VfsError::FileExist)
            } else {
                // otherwise, we create a new inode and insert it into dentry cache
                let file_inode = dentry.inode()?.create(file_name, ty, perm, None)?;
                let dir = dentry.insert(file_name, file_inode)?;
                Ok(dir)
            }
        } else {
            Err(VfsError::FileExist)
        }
    }

    /// Checks whether parent is a directory
    fn get_parent(&self, action: &str) -> VfsResult<Arc<dyn VfsDentry>> {
        let parent = self.parent();
        let parent = parent.exists()?;
        if parent.is_none() {
            error!("Could not {}, parent directory does not exist", action);
            return Err(VfsError::NoEntry);
        }
        let parent = parent.unwrap();
        if !parent.inode()?.inode_type().is_dir() {
            error!("Could not {}, parent path is not a directory", action);
            return Err(VfsError::NotDir);
        }
        Ok(parent)
    }
    pub fn parent(&self) -> Self {
        let index = self.path.rfind('/');
        index
            .map(|idx| VfsPath {
                path: self.path[..idx].to_string(),
                fs: self.fs.clone(),
            })
            .unwrap_or_else(|| self.root())
    }

    pub fn exists(&self) -> VfsResult<Option<Arc<dyn VfsDentry>>> {
        let mut parent = self.fs.clone();
        let mut path = self.path.as_str();
        loop {
            let (name, rest) = split_path(path);
            let parent_inode = parent.inode()?;

            // if the parent is not a dir, we return Err
            if !parent_inode.inode_type().is_dir() {
                return Err(VfsError::NotDir);
            }
            if name.is_empty() {
                break;
            }
            // resolve mount point
            let dentry = real_dentry(parent);
            // first, we find in dentry cache
            let sub_dentry = dentry.find(name);
            if sub_dentry.is_none() {
                // second, we find in inode cache or disk
                let sub_inode = parent_inode.lookup(name)?;
                if sub_inode.is_none() {
                    // if we can't find the inode, we return None
                    return Ok(None);
                }
                // if we find the inode, we insert it into dentry cache
                let sub_inode = sub_inode.unwrap();
                let sub_dentry = dentry.insert(name, sub_inode)?;
                parent = sub_dentry;
            } else {
                parent = sub_dentry.unwrap();
            }
            if rest.is_none() {
                break;
            }
            path = rest.unwrap();
        }
        // resolve mount point
        let dentry = real_dentry(parent);
        Ok(Some(dentry))
    }
    pub fn filename(&self) -> String {
        let index = self.path.rfind('/').map(|x| x + 1).unwrap_or(0);
        self.path[index..].to_string()
    }

    pub fn extension(&self) -> Option<String> {
        let filename = self.filename();
        let mut parts = filename.rsplitn(2, '.');
        let after = parts.next();
        let before = parts.next();
        match before {
            None | Some("") => None,
            _ => after.map(|x| x.to_string()),
        }
    }
}

fn real_dentry(dentry: Arc<dyn VfsDentry>) -> Arc<dyn VfsDentry> {
    if dentry.is_mount_point() {
        let mnt = dentry.mount_point().unwrap();
        real_dentry(mnt.root)
    } else {
        dentry
    }
}

fn split_path(path: &str) -> (&str, Option<&str>) {
    let trimmed_path = path.trim_start_matches('/');
    trimmed_path.find('/').map_or((trimmed_path, None), |n| {
        (&trimmed_path[..n], Some(&trimmed_path[n + 1..]))
    })
}

pub fn print_fs_tree(
    output: &mut dyn Write,
    root: Arc<dyn VfsDentry>,
    prefix: String,
) -> Result<(), Box<dyn Error>> {
    let mut children = root.inode()?.children();
    let mut child = children.next();
    while let Some(c) = child {
        let name = c.name;
        let inode_type = c.ty;
        let inode = root.inode()?.lookup(&name)?.unwrap();
        let stat = inode.get_attr()?;
        let perm = VfsNodePerm::from_bits_truncate(stat.st_mode as u16);
        let rwx_buf = perm.rwx_buf();
        let rwx = core::str::from_utf8(&rwx_buf)?;

        let mut buf = [0u8; 20];
        let option = if inode_type == VfsNodeType::SymLink {
            let r = inode.readlink(&mut buf)?;
            let content = core::str::from_utf8(&buf[..r])?;
            "-> ".to_string() + content
        } else {
            "".to_string()
        };

        write(
            output,
            format_args!(
                "{}{}{} {:>8} {} {}\n",
                prefix,
                inode_type.as_char(),
                rwx,
                stat.st_size,
                name,
                option
            ),
        )
        .unwrap();

        if inode_type == VfsNodeType::Dir {
            let d = root.find(&name);
            let sub_dt = if let Some(d) = d {
                d
            } else {
                let d = root.inode()?.lookup(&name)?.unwrap();

                root.i_insert(&name, d)?
            };
            if !sub_dt.is_mount_point() {
                print_fs_tree(output, sub_dt, prefix.clone() + "  ")?;
            } else {
                let mnt = sub_dt.mount_point().unwrap();
                let new_root = mnt.root;
                print_fs_tree(output, new_root, prefix.clone() + "  ")?;
            }
        }
        child = children.next();
    }
    Ok(())
}

trait DirIter {
    fn children(&self) -> Box<dyn Iterator<Item = VfsDirEntry>>;
}

struct DirIterImpl {
    inode: Arc<dyn VfsInode>,
    index: usize,
}
impl Iterator for DirIterImpl {
    type Item = VfsDirEntry;
    fn next(&mut self) -> Option<Self::Item> {
        let x = self.inode.readdir(self.index).unwrap();
        if let Some(x) = x {
            self.index += 1;
            Some(x)
        } else {
            None
        }
    }
}

impl DirIter for Arc<dyn VfsInode> {
    fn children(&self) -> Box<dyn Iterator<Item = VfsDirEntry>> {
        Box::new(DirIterImpl {
            inode: self.clone(),
            index: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::dentry::VfsDentry;
    use crate::fstype::{MountFlags, VfsMountPoint};
    use crate::inode::VfsInode;
    use crate::path::{split_path, VfsPath};
    use crate::VfsResult;
    use alloc::string::String;
    use alloc::sync::Arc;

    struct FakeDentry;
    impl VfsDentry for FakeDentry {
        fn name(&self) -> String {
            todo!()
        }

        fn to_mount_point(
            self: Arc<Self>,
            _sub_fs_root: Arc<dyn VfsDentry>,
            _mount_flag: MountFlags,
        ) -> VfsResult<()> {
            todo!()
        }

        fn inode(&self) -> VfsResult<Arc<dyn VfsInode>> {
            todo!()
        }

        fn mount_point(&self) -> Option<VfsMountPoint> {
            todo!()
        }

        fn clear_mount_point(&self) {
            todo!()
        }

        fn find(&self, _path: &str) -> Option<Arc<dyn VfsDentry>> {
            todo!()
        }

        fn insert(
            self: Arc<Self>,
            _name: &str,
            _child: Arc<dyn VfsInode>,
        ) -> VfsResult<Arc<dyn VfsDentry>> {
            todo!()
        }

        fn remove(&self, _name: &str) -> Option<Arc<dyn VfsDentry>> {
            todo!()
        }
    }

    #[test]
    fn test_split_path() {
        assert_eq!(split_path("/foo/bar.txt"), ("foo", Some("bar.txt")));
        assert_eq!(split_path("/foo/bar"), ("foo", Some("bar")));
        assert_eq!(split_path("/foo"), ("foo", None));
        assert_eq!(split_path("/"), ("", None));
        assert_eq!(split_path(""), ("", None));
    }

    #[test]
    fn test_join() {
        let path = VfsPath::new(Arc::new(FakeDentry));

        assert_eq!(path.join("foo.txt").unwrap().as_str(), "/foo.txt");
        assert_eq!(path.join("foo/bar.txt").unwrap().as_str(), "/foo/bar.txt");

        let foo = path.join("foo").unwrap();

        assert_eq!(
            path.join("foo/bar.txt").unwrap(),
            foo.join("bar.txt").unwrap()
        );
        assert_eq!(path, foo.join("..").unwrap());
    }
}
