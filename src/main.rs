#![no_main]
#![no_std]

mod boot_selector;
mod kernel_loader;
mod entries_parse;
extern crate alloc;
use alloc::vec::Vec;
use boot_selector::boot_menu;
use kernel_loader::load_efi_from_path;
use entries_parse::BootEntry;
use entries_parse::read_loader_entries;
use uefi::boot::{self, SearchType};
use uefi::prelude::*;
use uefi::println;
use uefi::proto::console::text::Input;
use uefi::proto::device_path::text::{AllowShortcuts, DevicePathToText, DisplayOnly};
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode, FileType};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{CStr16, Identify, Result};

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    println!("BOOTX64.EFI: Starting bootloader...");
    //print_image_path().unwrap();
    println!("\n\n");
    let handle = *boot::locate_handle_buffer(SearchType::ByProtocol(&Input::GUID))
        .unwrap()
        .first()
        .expect("No handle supports TextInput protocol");
    let mut input = boot::open_protocol_exclusive::<Input>(handle).unwrap();
    let entries = read_loader_entries().unwrap();

    if let Ok(Some(entry)) = boot_menu(&entries, &mut input) {
        if let Some(path_linux) = entry.linux {
            load_efi_from_path(&path_linux, None, None).unwrap();
        }
            /*match load_kernel_image(
                &path_linux,
                entry.initrd.as_deref(),
                entry.options.as_deref(),
            )
            {
                Ok(val) => println!("Success: {:?}", val),
                Err(e) => {
                    println!("❌ Failed to load kernel image:");
        println!("   kernel: {:?}", path_linux);
        println!("   initrd: {:?}", entry.initrd);
        println!("   options: {:?}", entry.options);
        println!("   error: {}", e);
                },
            }
        } else if let Some(path_efi) = entry.efi {
            load_efi_from_path(&path_efi).unwrap();
        }*/
        //load_efi_from_path(&selected).unwrap();
    }

    boot::stall(100_000_000);
    uefi::runtime::reset(uefi::runtime::ResetType::COLD, Status::SUCCESS, None);
    //Status::SUCCESS
}

/*fn print_image_path() -> Result {
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
}*/

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
