use std::sync::Arc;
use vfscore::dentry::VfsDentry;
use vfscore::inode::VfsInode;
use vfscore::utils::{VfsDirEntry, VfsNodeType};

pub fn print_fs_tree(root: Arc<dyn VfsDentry>, prefix: String) {
    let mut children = root.inode().unwrap().children();
    let mut child = children.next();
    while let Some(c) = child {
        let name = c.name;
        let inode = c.ino;
        let inode_type = c.ty;
        let inode_type = match inode_type {
            VfsNodeType::File => "File",
            VfsNodeType::Dir => "Dir",
            VfsNodeType::SymLink => "SymLink",
            VfsNodeType::CharDevice => "CharDevice",
            VfsNodeType::BlockDevice => "BlockDevice",
            VfsNodeType::Socket => "Socket",
            VfsNodeType::Fifo => "Fifo",
            VfsNodeType::Unknown => "Unknown",
        };
        println!("{}{} ({}) ({})", prefix, name, inode_type,inode);
        if inode_type == "Dir" {
            let d= root.find(&name);
            let sub_dt = if let Some(d) = d{
                d
            }else {
                let d= root.inode().unwrap().lookup(&name).unwrap().unwrap();
                let d = root.clone().insert(&name,d).unwrap();
                d
            };
            if !sub_dt.is_mount_point(){
                print_fs_tree(sub_dt, prefix.clone() + "  ")
            }else {
                let mnt = sub_dt.get_vfs_mount().unwrap();
                let new_root = mnt.root;
                print_fs_tree(new_root, prefix.clone() + "  ")
            }
        }
        child = children.next();
    }
}


trait DirIter{
    fn children(&self)->Box<dyn Iterator<Item=VfsDirEntry>>;
}

struct DirIterImpl{
    inode:Arc<dyn VfsInode>,
    index:usize,
}
impl Iterator for DirIterImpl{
    type Item=VfsDirEntry;
    fn next(&mut self)->Option<Self::Item>{
        let x = self.inode.readdir(self.index).unwrap();
        if let Some(x) = x{
            self.index+=1;
            Some(x)
        } else{
            None
        }
    }
}


impl DirIter for Arc<dyn VfsInode>{
    fn children(&self)->Box<dyn Iterator<Item=VfsDirEntry>>{
        Box::new(DirIterImpl{
            inode:self.clone(),
            index:0,
        })
    }
}