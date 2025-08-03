// loader_entries.rs
// Module to read systemd-boot style entries from ESP
mod fs_handler;
use alloc::fmt::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::str;
use uefi::proto::media::file::{File, FileAttribute, FileMode, FileType};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{prelude::*, println};
//use uefi::proto::media::fs::Directory;
use crate::alloc::string::ToString;
use uefi::CStr16;
use uefi::Result;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::Directory;

#[derive(Debug, Clone)]
pub struct BootEntry {
    pub title: String,
    pub version: Option<String>,
    pub machine_id: Option<String>,
    pub sort_key: Option<String>,
    pub linux: Option<String>,
    pub initrd: Option<String>,
    pub efi: Option<String>,
    pub options: Option<String>,
    
}

impl BootEntry {
    pub fn new() -> Self {
        BootEntry {
            title: String::new(),
            version: None,
            machine_id: None,
            sort_key: None,
            linux: None,
            initrd: None,
            efi: None,
            options: None, 
        }
    }
}

/// Reads all .conf files under /loader/entries and returns parsed BootEntry list.
pub fn read_loader_entries() -> Result<Vec<BootEntry>> {
    //st: &SystemTable<Boot>
    // Open the SimpleFileSystem for the loaded image's device
    /*let loaded_image = st.boot_services().handle_protocol::<uefi::proto::loaded_image::LoadedImage>(
    st.boot_services().image_handle())?
    .interface;*/
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())?;
    println!("Line 1: {:?}", loaded_image);
    //let device = unsafe { (*loaded_image.get()).device() };
    let device_handle = loaded_image.device();
    //println!("Line2");
    /*let fs = st.boot_services().handle_protocol::<SimpleFileSystem>(device.unwrap())?
    .interface;*/
    let mut sfs = boot::open_protocol_exclusive::<SimpleFileSystem>(device_handle.unwrap())?;
    //let mut root = unsafe { (*fs.get()).open_volume()? };

    // Open the root directory
    let mut folder = sfs.open_volume()?;
    println!("Line3");

    // Navigate to \loader\entries for conf detection
    open_dir(&mut folder, "loader")?;
    open_dir(&mut folder, "entries")?;
    let buf: &mut [u8] = &mut [0; 10000];

    let mut entries = Vec::new();
    loop {
        println!("I ahev entered the loop");
        //let x = folder.read_entry(buf);
        //println!("Rsult = {x:#?}")
        match folder.read_entry(buf) {
            Err(e) => println!("Error: {e}"),
            core::prelude::v1::Ok(None) => break,
            core::prelude::v1::Ok(Some(file_info)) => {
                let name = file_info.file_name();
                println!("should execute rn");
                if !name.to_string().ends_with(".conf") {
                    println!("Should not executed rihgtnow");
                    continue;
                }
                // Open file
                let file_handle = folder.open(
                    //unsafe { CStr16::from_u16_with_nul_unchecked(&to_utf16(&name)) },
                    &name,
                    FileMode::Read,
                    FileAttribute::empty(),
                )?;
                let mut file = match file_handle.into_type()? {
                    FileType::Regular(f) => f,
                    _ => continue,
                };
                // Read content into buffer
                let mut buf: &mut [u8] = &mut [0; 1000];
                file.read(&mut buf)?;
                if let Ok(text) = str::from_utf8(&buf) {
                    let entry = parse_conf(text);
                    // Ensure mandatory fields
                    //if !entry.title.is_empty() && !entry.linux.is_empty() {
                    entries.push(entry);
                    //}
                }
            }
        }
    }
    //Add autodetect for Windows and macOS
    let mut root = sfs.open_volume()?;
    for path in [
        "EFI\\Microsoft\\Boot\\bootmgfw.efi",
        "EFI\\Apple\\Boot\\boot.efi",
        "shellx64.efi",
    ] {
        //if let Ok(file_handle) = try_open_path(&mut root, path) {
        if try_open_path(&mut root, path).unwrap() {
            entries.push(BootEntry {
                title: format(format_args!("Detected Boot Entry: {}", path)),
                version: None,
                machine_id: None,
                sort_key: None,
                linux: None,
                initrd: None,
                efi: Some(path.into()),
                options: None,
            });
        }
    }
    //Add \EFI\Linux kernel detection
    if let Some(mut linux_dir) = fs_handler::open_dir(&mut root)? {
        // Proceed to enumerate EFI/Linux kernels
        loop {
            let file = match linux_dir.read_entry(buf).unwrap() {
                Some(info) => info,
                None => break,
            };

            let name = file.file_name().to_string();
            
            entries.push(BootEntry {
                title: format(format_args!("Linux EFI Kernel: {}", name)),
                version: None,
                machine_id: None,
                sort_key: None,
                linux: None,
                initrd: None,
                efi: Some(format(format_args!("EFI\\Linux\\{}", name))),
                options: None,
            });
        }
    } else {
        println!("Skipping EFI/Linux kernel detection.");
    }
    Ok(entries)
}

pub fn try_open_path(root: &mut Directory, path: &str) -> Result<bool> {
    // Convert path to UTF-16 with null terminator
    let mut path_utf16 = [0u16; 260];
    let mut len = 0;
    for ch in path.chars() {
        if len >= path_utf16.len() - 1 {
            return Err(uefi::Error::new(Status::BUFFER_TOO_SMALL, ()));
        }
        path_utf16[len] = ch as u16;
        len += 1;
    }
    path_utf16[len] = 0;

    // Attempt to open the file
    match root.open(
        unsafe { CStr16::from_u16_with_nul_unchecked(&path_utf16[..=len]) },
        FileMode::Read,
        FileAttribute::empty(),
    ) {
        Ok(file_handle) => {
            match file_handle.into_type()? {
                FileType::Regular(_) => Ok(true),
                FileType::Dir(_) => Ok(false), // It's a dir, not a file
            }
        }
        Err(e) if e.status() == Status::NOT_FOUND => Ok(false),
        Err(e) => Err(e),
    }
}

/// Open (and move into) a subdirectory by name
fn open_dir(dir: &mut Directory, name: &str) -> Result<()> {
    let mut name_utf16 = [0u16; 256];
    let mut i = 0;
    for ch in name.encode_utf16() {
        if i >= 255 {
            break;
        }
        name_utf16[i] = ch;
        i += 1;
    }
    name_utf16[i] = 0;
    let handle = dir.open(
        unsafe { CStr16::from_u16_with_nul_unchecked(&name_utf16[..=i]) },
        FileMode::Read,
        FileAttribute::empty(),
    )?;
    let subdir = match handle.into_type()? {
        FileType::Dir(d) => d,
        _ => return Err(uefi::Error::new(uefi::Status::NOT_FOUND, ())),
    };
    *dir = subdir;
    Ok(())
}

/// Parse a single .conf text into BootEntry
fn parse_conf(text: &str) -> BootEntry {
    let mut entry = BootEntry::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(idx) = line.find(' ') {
            let (key, val) = line.split_at(idx);
            let val = val.trim_start();
            match key {
                "title" => entry.title = val.to_string(),
                "sort-key" => entry.sort_key = Some(val.to_string()),
                "version" => entry.version = Some(val.to_string()),
                "linux" => entry.linux = Some(val.to_string()),
                "initrd" => entry.initrd = Some(val.to_string()),
                "options" => entry.options = Some(val.to_string()),
                "machine-id" => entry.machine_id = Some(val.to_string()),
                _ => (),
            }
        }
    }
    entry
}
