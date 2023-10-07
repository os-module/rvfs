use std::error::Error;

use crate::procfs::init_procfs;
use crate::ramfs::init_ramfs;

mod procfs;
mod ramfs;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    init_procfs()?;
    init_ramfs()?;
    let path = std::path::Path::new("./ramfs");
    let file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    println!("{:#?}", meta);

    // std::path::Component::CurDir;

    // let path = VfsPath::new

    Ok(())
}
