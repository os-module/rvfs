use std::error::Error;
use std::sync::Arc;
use vfscore::dentry::VfsDentry;
use vfscore::inode::VfsInode;
use vfscore::utils::{VfsDirEntry, VfsNodePerm, VfsNodeType};

pub fn print_fs_tree(root: Arc<dyn VfsDentry>, prefix: String) -> Result<(), Box<dyn Error>> {
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

        println!(
            "{}{}{} {:>8} {} {}",
            prefix,
            inode_type.as_char(),
            rwx,
            stat.st_size,
            name,
            option
        );

        if inode_type == VfsNodeType::Dir {
            let d = root.find(&name);
            let sub_dt = if let Some(d) = d {
                d
            } else {
                let d = root.inode()?.lookup(&name)?.unwrap();
                
                root.i_insert(&name, d)?
            };
            if !sub_dt.is_mount_point() {
                print_fs_tree(sub_dt, prefix.clone() + "  ")?;
            } else {
                let mnt = sub_dt.mount_point().unwrap();
                let new_root = mnt.root;
                print_fs_tree(new_root, prefix.clone() + "  ")?;
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
