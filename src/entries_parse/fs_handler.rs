use uefi::proto::media::file::{Directory, File, FileAttribute, FileMode, FileType};
use uefi::{CStr16, Result, Status, cstr16, println};

pub fn open_dir(root: &mut Directory) -> Result<Option<uefi::proto::media::file::Directory>> {
    match root.open(
        cstr16!("EFI\\Linux"),
        FileMode::Read,
        FileAttribute::empty(),
    ) {
        Ok(handle) => match handle.into_type()? {
            FileType::Dir(dir) => Ok(Some(dir)),
            FileType::Regular(_) => {
                println!("EFI\\Linux exists but is not a directory.");
                Ok(None)
            }
        },
        Err(e) if e.status() == Status::NOT_FOUND => {
            println!("EFI\\Linux directory not found.");
            Ok(None)
        }
        Err(e) => {
            println!("Failed to open EFI\\Linux: {:?}", e.status());
            Err(e)
        }
    }
}
pub fn _open_file(root: &mut Directory, filename: &str) -> Result<Option<uefi::proto::media::file::RegularFile>> {
    let mut buf = [0; 400];
    match root.open(
        CStr16::from_str_with_buf(filename, &mut buf).unwrap(),
        FileMode::Read,
        FileAttribute::empty(),
    ) {
        Ok(handle) => match handle.into_type()? {
            FileType::Regular(dir) => Ok(Some(dir)),
            FileType::Dir(_) => {
                println!("filepath exists but is a directory.");
                Ok(None)
            }
        },
        Err(e) if e.status() == Status::NOT_FOUND => {
            println!("filepath not found.");
            Ok(None)
        }
        Err(e) => {
            println!("Failed to open filepath: {:?}", e.status());
            Err(e)
        }
    }
}
