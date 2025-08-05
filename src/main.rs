#![no_main]
#![no_std]

mod loader_entries;
mod boot_selector;
extern crate alloc;
use loader_entries::BootEntry;
use loader_entries::read_loader_entries;
use boot_selector::boot_menu;
use alloc::vec::Vec;
use uefi::boot::{self, SearchType};
use uefi::prelude::*;
use uefi::proto::device_path::text::{
    AllowShortcuts, DevicePathToText, DisplayOnly,
};
use uefi::proto::console::text::Input;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{File, FileAttribute, FileMode, FileInfo, FileType};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{Identify, Result, CStr16};
use uefi::println;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    
    println!("BOOTX64.EFI: Starting bootloader...");
    print_image_path().unwrap();
    println!("\n\n");
    let handle = *boot::locate_handle_buffer(SearchType::ByProtocol(&Input::GUID))
        .unwrap()
        .first()
        .expect("No handle supports TextInput protocol");
    let mut input = boot::open_protocol_exclusive::<Input>(handle).unwrap();
    let entries = read_loader_entries().unwrap();

    if let Ok(Some(selected)) = boot_menu(&entries, &mut input) {
        load_efi_from_path(&selected).unwrap();
    }
    
    //boot::stall(100_000_000);
    uefi::runtime::reset(uefi::runtime::ResetType::COLD, Status::SUCCESS, None);
    //Status::SUCCESS
}

fn print_image_path() -> Result {
    let loaded_image =
        boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())?;
    let device_path_to_text_handle = *boot::locate_handle_buffer(
        SearchType::ByProtocol(&DevicePathToText::GUID),
    )?
    .first()
    .expect("DevicePathToText is missing");
    let device_path_to_text = boot::open_protocol_exclusive::<DevicePathToText>(
        device_path_to_text_handle,
    )?;
    let image_device_path =
        loaded_image.file_path().expect("File path is not set");
    let image_device_path_text = device_path_to_text
        .convert_device_path_to_text(
            image_device_path,
            DisplayOnly(true),
            AllowShortcuts(false),
        )
        .expect("convert_device_path_to_text failed");
    println!("Image path: {:?}", &*image_device_path_text);
    Ok(())
}

fn load_efi_from_path(kernel_path: &str) -> Result {
    // Get the loaded image protocol for the current image (BOOTX64.EFI)
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())?;
    
    // Get the device handle where this image was loaded from
    let device_handle = loaded_image.device();
    
    // Open the Simple File System protocol on the same device
    let mut sfs = boot::open_protocol_exclusive::<SimpleFileSystem>(device_handle.unwrap())?;
    
    // Open the root directory
    let mut current_dir = sfs.open_volume()?;
    
    println!("Loading kernel from path: {}", kernel_path);
    
    // Split the path and navigate to the correct directory
    let kernel_path = kernel_path.replace('/', "\\");
    let path_parts: Vec<&str> = kernel_path.split('\\')//.filter(|part| !part.is_empty())
        .collect();
    let filename = path_parts.last().unwrap();
    
    // Navigate through directories if path has subdirectories
    for &dir_name in &path_parts[..path_parts.len() - 1] {
        if !dir_name.is_empty() {
            println!("Navigating to directory: {}", dir_name);
            
            // Convert to UTF-16 string
            let mut dir_name_utf16 = [0u16; 256];
            let mut i = 0;
            for ch in dir_name.chars() {
                if i >= 255 { break; }
                dir_name_utf16[i] = ch as u16;
                i += 1;
            }
            dir_name_utf16[i] = 0; // Null terminator
            
            let dir_handle = current_dir.open(
                unsafe { CStr16::from_u16_with_nul_unchecked(&dir_name_utf16[..=i]) },
                FileMode::Read,
                FileAttribute::empty(),
            )?;
            
            current_dir = match dir_handle.into_type()? {
                FileType::Dir(dir) => dir,
                FileType::Regular(_) => {
                    println!("{} is not a directory", dir_name);
                    return Err(uefi::Error::new(uefi::Status::INVALID_PARAMETER, ()));
                }
            };
        }
    }

    // Convert filename to UTF-16
    let mut filename_utf16 = [0u16; 256];
    let mut i = 0;
    for ch in filename.chars() {
        if i >= 255 { break; }
        filename_utf16[i] = ch as u16;
        i += 1;
    }
    filename_utf16[i] = 0; // Null terminator
    
    // Open the kernel file
    let kernel_file_handle = current_dir.open(
        unsafe { CStr16::from_u16_with_nul_unchecked(&filename_utf16[..=i]) },
        FileMode::Read,
        FileAttribute::empty(),
    )?;
    
    let mut kernel_file = match kernel_file_handle.into_type()? {
        FileType::Regular(file) => file,
        FileType::Dir(_) => {
            println!("{} is a directory, not a file", filename);
            return Err(uefi::Error::new(uefi::Status::INVALID_PARAMETER, ()));
        }
    };
    
    // Get file info to determine size
    let mut info_buffer = [0u8; 200]; // Should be enough for FileInfo
    let file_info = kernel_file.get_info::<FileInfo>(&mut info_buffer);
    let file_size = file_info.unwrap().file_size() as usize;
    
    println!("{} size: {} bytes", filename, file_size);
    
    // Allocate memory for the kernel image
    let kernel_pages = (file_size + 4095) / 4096; // Round up to page boundary
    let kernel_addr = boot::allocate_pages(
        boot::AllocateType::AnyPages,
        boot::MemoryType::LOADER_DATA,
        kernel_pages,
    )?;
    
    // Read the kernel file into memory
    let kernel_buffer = unsafe {
        core::slice::from_raw_parts_mut(kernel_addr.as_ptr(), file_size)
    };
    
    kernel_file.read(kernel_buffer)?;
    println!("{} loaded into memory at 0x{:x}", filename, kernel_addr.as_ptr() as usize);
    
    // Load the image
    let kernel_image_handle = boot::load_image(
        boot::image_handle(),
        boot::LoadImageSource::FromBuffer {
            buffer: kernel_buffer,
            file_path: None,
        },
    )?;
    
    println!("{} image loaded, starting execution...", filename);
    
    // Start the kernel image
    boot::start_image(kernel_image_handle)?;
    
    // If we reach here, the kernel returned (which might not be expected)
    Ok(())
}

/*
fn load_kernel() -> Result {
    // Get the loaded image protocol for the current image (BOOTX64.EFI)
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())?;
    
    // Get the device handle where this image was loaded from
    let device_handle = loaded_image.device();
    
    // Open the Simple File System protocol on the same device
    let mut sfs = boot::open_protocol_exclusive::<SimpleFileSystem>(device_handle.unwrap())?;
    
    // Open the root directory
    let mut root_dir = sfs.open_volume()?;
    
    // Open KERNEL.EFI file
    let kernel_file_handle = root_dir.open(
        cstr16!("KERNEL.EFI"),
        FileMode::Read,
        FileAttribute::empty(),
    )?;
    
    let mut kernel_file = match kernel_file_handle.into_type()? {
        FileType::Regular(file) => file,
        FileType::Dir(_) => {
            println!("KERNEL.EFI is a directory, not a file");
            return Err(uefi::Error::new(uefi::Status::INVALID_PARAMETER, ()));
        }
    };
    
    // Get file info to determine size
    let mut info_buffer = [0u8; 200]; // Should be enough for FileInfo
    let file_info = kernel_file.get_info::<FileInfo>(&mut info_buffer);
    let file_size = file_info.unwrap().file_size() as usize;
    
    println!("KERNEL.EFI size: {} bytes", file_size);
    
    // Allocate memory for the kernel image
    let kernel_pages = (file_size + 4095) / 4096; // Round up to page boundary
    let kernel_addr = boot::allocate_pages(
        boot::AllocateType::AnyPages,
        boot::MemoryType::LOADER_DATA,
        kernel_pages,
    )?;
    
    // Read the kernel file into memory
    let kernel_buffer = unsafe {
        core::slice::from_raw_parts_mut(kernel_addr.as_ptr(), file_size)
    };
    
    
    kernel_file.read(kernel_buffer)?;
    println!("KERNEL.EFI loaded into memory at 0x{:x}", kernel_addr.as_ptr() as usize);
    
    // Load the image
    let kernel_image_handle = boot::load_image(
        boot::image_handle(),
        boot::LoadImageSource::FromBuffer {
            buffer: kernel_buffer,
            file_path: None,
        },
    )?;
    
    println!("KERNEL.EFI image loaded, starting execution...");
    
    // Start the kernel image
    boot::start_image(kernel_image_handle)?;
    
    // If we reach here, the kernel returned (which might not be expected)
    Ok(())
}
*/