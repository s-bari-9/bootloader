// kernel_loader.rs
// Loads a Linux bzImage and jumps to it from UEFI

use uefi::{boot, Result};
use uefi::proto::media::{fs::SimpleFileSystem, file::{File,FileMode,FileAttribute,FileType,FileInfo}};
use uefi::proto::loaded_image::LoadedImage;
use uefi::println;
use uefi::CStr16;
use alloc::vec::Vec;


pub fn load_efi_from_path(
    kernel_path: &str,
    initrd_path: Option<&str>,
    cmdline: Option<&str>) -> Result {
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
    let path_parts: Vec<&str> = kernel_path
        .split('\\') //.filter(|part| !part.is_empty())
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
                if i >= 255 {
                    break;
                }
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
        if i >= 255 {
            break;
        }
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
    let kernel_buffer = unsafe { core::slice::from_raw_parts_mut(kernel_addr.as_ptr(), file_size) };

    kernel_file.read(kernel_buffer)?;
    println!(
        "{} loaded into memory at 0x{:x}",
        filename,
        kernel_addr.as_ptr() as usize
    );

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
extern crate alloc;

mod memory;
use memory::{convert_memory_map_to_e820, validate_e820_map, print_e820_map, install_e820_map};
use alloc::vec::Vec;
use uefi::println;
use core::{ptr, slice};
use uefi::prelude::*;
use uefi::boot;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::proto::media::file::{File, FileMode, FileType, FileInfo, FileAttribute};
use uefi::{CStr16, Result};

const KERNEL_LOAD_ADDR: u64 = 0x100000;
const BOOT_PARAMS_ADDR: usize = 0x90000;
const SETUP_SECTS_OFFSET: usize = 0x1F1;
const HEADER_MAGIC_OFFSET: usize = 0x202;
const HEADER_MAGIC: u32 = 0x53726448; // "HdrS"

#[repr(C, packed)]
struct SetupHeader {
    setup_sects: u8,
    root_flags: u16,
    syssize: u32,
    ramdisk_image: u32,
    ramdisk_size: u32,
    cmd_line_ptr: u32,
    type_of_loader: u8,
    loadflags: u8,
    setup_move_size: u16,
    code32_start: u32,
    ramdisk_max: u32,
    kernel_alignment: u32,
    relocatable_kernel: u8,
    min_alignment: u8,
    cmdline_size: u16,
    hardware_subarch: u32,
    payload_offset: u64,
    payload_length: u64,
    pref_address: u64,
    init_size: u32,
    handover_offset: u32,
    kernel_info_offset: u32,
}

pub fn load_kernel_image(
    kernel_path: &str,
    initrd_path: Option<&str>,
    cmdline: Option<&str>,
) -> Result<()> {
    println!("{}", kernel_path);
    let kernel_data = read_file_to_vec(kernel_path)?;//163b44d200684e49ada2859de61dbaad/6.13.7-arch1-1/linux
    println!("aa");

    // Check Linux boot protocol header magic
    if kernel_data.len() < HEADER_MAGIC_OFFSET + 4 {
        panic!("Kernel image too small");
    }
    
    let hdr_magic = u32::from_le_bytes([
        kernel_data[HEADER_MAGIC_OFFSET],
        kernel_data[HEADER_MAGIC_OFFSET + 1],
        kernel_data[HEADER_MAGIC_OFFSET + 2],
        kernel_data[HEADER_MAGIC_OFFSET + 3],
    ]);
    println!("debug");
    if hdr_magic != HEADER_MAGIC {
        panic!("Invalid kernel header magic: 0x{:08x}, expected 0x{:08x}", hdr_magic, HEADER_MAGIC);
    }

    // Load kernel image to 0x100000
    let pages = (kernel_data.len() + 0xFFF) / 0x1000;
    /*let kernel_addr = boot::allocate_pages(
        boot::AllocateType::Address(KERNEL_LOAD_ADDR),
        boot::MemoryType::LOADER_DATA,
        pages,
    ).unwrap();*/
let kernel_addr = boot::allocate_pages(
        boot::AllocateType::AnyPages,//Address(KERNEL_LOAD_ADDR),
        boot::MemoryType::LOADER_DATA,
        pages,
    ).unwrap();
    println!("yay");
    
    let kernel_entry = kernel_addr.as_ptr() as u64 + 0x200;

    // Verify we got the address we requested
    if kernel_entry != KERNEL_LOAD_ADDR {
        println!("Failed to allocate kernel at requested address 0x{:x}, got 0x{:x}", 
               KERNEL_LOAD_ADDR, kernel_entry);
    }
    
    unsafe {
        ptr::copy_nonoverlapping(kernel_data.as_ptr(), KERNEL_LOAD_ADDR as *mut u8, kernel_data.len());
    }

    // Load initrd
    let (initrd_addr, initrd_size) = if let Some(path) = initrd_path {
        println!("hi");
        let initrd = read_file_to_vec(path)?;
        println!("hello");
        let pages = (initrd.len() + 0xFFF) / 0x1000;
        let addr = boot::allocate_pages(
            boot::AllocateType::AnyPages,
            boot::MemoryType::LOADER_DATA,
            pages,
        )?;
        unsafe {
            ptr::copy_nonoverlapping(initrd.as_ptr(), addr.as_ptr(), initrd.len());
        }
        (addr.as_ptr() as u64, initrd.len())
    } else {
        (0, 0)
    };

    // Setup boot_params structure at 0x90000
    let boot_params_pages = (4096 + 0xFFF) / 0x1000;
    let boot_params_addr = boot::allocate_pages(
        boot::AllocateType::Address(BOOT_PARAMS_ADDR as u64),
        boot::MemoryType::LOADER_DATA,
        boot_params_pages,
    )?;
    
    // Verify we got the address we requested
    if boot_params_addr.as_ptr() as usize != BOOT_PARAMS_ADDR {
        panic!("Failed to allocate boot_params at requested address 0x{:x}, got 0x{:x}",
               BOOT_PARAMS_ADDR, boot_params_addr.as_ptr() as usize);
    }
    
    let boot_params = BOOT_PARAMS_ADDR as *mut u8;
    unsafe {
        ptr::write_bytes(boot_params, 0, 4096);
        // Copy first 0x1f1 bytes from kernel image
        if kernel_data.len() >= 0x1F1 {
            ptr::copy_nonoverlapping(kernel_data.as_ptr(), boot_params, 0x1F1);
        } else {
            ptr::copy_nonoverlapping(kernel_data.as_ptr(), boot_params, kernel_data.len());
        }
    }

    // Add initrd and cmdline pointers to setup header
    if kernel_data.len() >= 0x1F1 + core::mem::size_of::<SetupHeader>() {
        let setup_hdr = unsafe { &mut *(boot_params.add(0x1F1) as *mut SetupHeader) };
        setup_hdr.ramdisk_image = initrd_addr as u32;
        setup_hdr.ramdisk_size = initrd_size as u32;

        if let Some(cmdline_str) = cmdline {
            let bytes = cmdline_str.as_bytes();
            let cmdline_pages = (bytes.len() + 1 + 0xFFF) / 0x1000;
            let cmdline_addr = boot::allocate_pages(
                boot::AllocateType::AnyPages,
                boot::MemoryType::LOADER_DATA,
                cmdline_pages,
            )?;
            unsafe {
                let dst = cmdline_addr.as_ptr();
                ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
                *dst.add(bytes.len()) = 0;
            }
            setup_hdr.cmd_line_ptr = cmdline_addr.as_ptr() as u32;
        }
    }

    // Exit boot services
    //let (_st, _map_key) = unsafe {boot::exit_boot_services()};
    let memory_map = unsafe {boot::exit_boot_services(None)};

    // Convert UEFI memory map to E820 format
    let e820_entries = convert_memory_map_to_e820(&memory_map)
        .expect("Failed to convert memory map to E820 format");
    
    // Validate the E820 map
    validate_e820_map(&e820_entries)
        .expect("Invalid E820 memory map");
    
    // Print E820 map for debugging (optional)
    print_e820_map(&e820_entries);
    
    // Install E820 map into boot_params structure
    unsafe {
        install_e820_map(boot_params, &e820_entries)
            .expect("Failed to install E820 map into boot_params");
    }
    
    // Jump to kernel entry
    //let kernel_entry = KERNEL_LOAD_ADDR + 0x200; // Entry point offset for bzImage
    
    // This is where the kernel takes over - we can't return from here
    unsafe {
        let entry_fn: extern "C" fn() -> ! = core::mem::transmute(kernel_entry as *const ());
        entry_fn()
    }
}

/// Helper to read a file into a Vec<u8> using the newer UEFI crate API
fn read_file_to_vec(path: &str) -> Result<Vec<u8>> {
    // Get the loaded image protocol for the current image
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).unwrap();
    
    // Get the device handle where this image was loaded from
    let device_handle = loaded_image.device().unwrap();
    
    // Open the Simple File System protocol on the same device
    let mut sfs = boot::open_protocol_exclusive::<SimpleFileSystem>(device_handle).unwrap();
    
    // Open the root directory
    let mut current_dir = sfs.open_volume().unwrap();
    
    // Normalize path separators - replace forward slashes with backslashes
    let normalized_path = path.replace('/', "\\");
    
    // Split the path and navigate to the correct directory
    let path_parts: Vec<&str> = normalized_path.split('\\')
        .filter(|part| !part.is_empty())
        .collect();
    
    if path_parts.is_empty() {
        panic!("Empty file path: {}", path);
    }
    
    let filename = path_parts.last().unwrap();
    
    // Navigate through directories if path has subdirectories
    for &dir_name in &path_parts[..path_parts.len() - 1] {
        // Convert to UTF-16 string
        let mut dir_name_utf16 = [0u16; 256];
        let mut utf16_len = 0;
        
        for ch in dir_name.chars() {
            if utf16_len >= 255 {
                panic!("Directory name too long: {}", dir_name);
            }
            dir_name_utf16[utf16_len] = ch as u16;
            utf16_len += 1;
        }
        dir_name_utf16[utf16_len] = 0;
        
        let dir_handle = current_dir.open(
            unsafe { CStr16::from_u16_with_nul_unchecked(&dir_name_utf16[..=utf16_len]) },
            FileMode::Read,
            FileAttribute::empty(),
        ).unwrap();
        
        current_dir = match dir_handle.into_type()? {
            FileType::Dir(dir) => dir,
            FileType::Regular(_) => panic!("{} is not a directory", dir_name),
        };
    }

    // Convert filename to UTF-16
    let mut filename_utf16 = [0u16; 256];
    let mut utf16_len = 0;
    
    for ch in filename.chars() {
        if utf16_len >= 255 {
            panic!("Filename too long: {}", filename);
        }
        filename_utf16[utf16_len] = ch as u16;
        utf16_len += 1;
    }
    filename_utf16[utf16_len] = 0;

    // Open the file
    let handle = current_dir.open(
        unsafe { CStr16::from_u16_with_nul_unchecked(&filename_utf16[..=utf16_len]) },
        FileMode::Read,
        FileAttribute::empty(),
    ).unwrap();
    
    let mut file = match handle.into_type()? {
        FileType::Regular(f) => f,
        FileType::Dir(_) => panic!("{} is not a regular file", filename),
    };

    // Get file size
    let mut info_buffer = [0u8; 200];
    let file_info = file.get_info::<FileInfo>(&mut info_buffer).expect("Failed to get file info");
    let file_size = file_info.file_size() as usize;
    
    if file_size == 0 {
        panic!("File {} is empty", filename);
    }
    
    // Read file into vector
    let mut buf = Vec::with_capacity(file_size);
    let mut total_read = 0;
    
    while total_read < file_size {
        let mut chunk = [0u8; 4096];
        let bytes_to_read = core::cmp::min(chunk.len(), file_size - total_read);
        let bytes_read = file.read(&mut chunk[..bytes_to_read]).unwrap();
        
        if bytes_read == 0 {
            break; // EOF reached
        }
        
        buf.extend_from_slice(&chunk[..bytes_read]);
        total_read += bytes_read;
    }
    
    if total_read != file_size {
        panic!("Failed to read complete file {}: expected {} bytes, got {} bytes", 
               filename, file_size, total_read);
    }
    
    Ok(buf)
}

/*
extern crate alloc;

use alloc::vec::Vec;
use core::{ptr, slice};
use uefi::prelude::*;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::proto::media::file::{File, FileMode, FileType};
//use uefi::table::boot::{AllocateType, MemoryType};
use uefi::boot::{AllocateType, MemoryType};
use uefi::CStr16;

const KERNEL_LOAD_ADDR: u64 = 0x100000;
const BOOT_PARAMS_ADDR: usize = 0x90000;
const SETUP_SECTS_OFFSET: usize = 0x1F1;
const HEADER_MAGIC_OFFSET: usize = 0x202;
const HEADER_MAGIC: u32 = 0x53726448; // "HdrS"

#[repr(C, packed)]
struct SetupHeader {
    setup_sects: u8,
    root_flags: u16,
    syssize: u32,
    ramdisk_image: u32,
    ramdisk_size: u32,
    cmd_line_ptr: u32,
    type_of_loader: u8,
    loadflags: u8,
    setup_move_size: u16,
    code32_start: u32,
    ramdisk_max: u32,
    kernel_alignment: u32,
    relocatable_kernel: u8,
    min_alignment: u8,
    cmdline_size: u16,
    hardware_subarch: u32,
    payload_offset: u64,
    payload_length: u64,
    pref_address: u64,
    init_size: u32,
    handover_offset: u32,
    kernel_info_offset: u32,
}

pub fn load_kernel_image(
    st: &SystemTable<Boot>,
    kernel_path: &str,
    initrd_path: Option<&str>,
    cmdline: Option<&str>,
) -> Result<!> {
    let kernel_data = read_file_to_vec(st, kernel_path)?;

    // Check Linux boot protocol header magic
    let hdr_magic = u32::from_le_bytes([
        kernel_data[HEADER_MAGIC_OFFSET],
        kernel_data[HEADER_MAGIC_OFFSET + 1],
        kernel_data[HEADER_MAGIC_OFFSET + 2],
        kernel_data[HEADER_MAGIC_OFFSET + 3],
    ]);
    if hdr_magic != HEADER_MAGIC {
        panic!("Invalid kernel header magic");
    }

    // Load kernel image to 0x100000
    let pages = (kernel_data.len() + 0xFFF) / 0x1000;
    let kernel_addr = st.boot_services().allocate_pages(
        AllocateType::Address(KERNEL_LOAD_ADDR),
        MemoryType::LOADER_DATA,
        pages,
    )?;
    unsafe {
        ptr::copy_nonoverlapping(kernel_data.as_ptr(), KERNEL_LOAD_ADDR as *mut u8, kernel_data.len());
    }

    // Load initrd
    let (initrd_addr, initrd_size) = if let Some(path) = initrd_path {
        let initrd = read_file_to_vec(st, path)?;
        let pages = (initrd.len() + 0xFFF) / 0x1000;
        let addr = st.boot_services().allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            pages,
        )?;
        unsafe {
            ptr::copy_nonoverlapping(initrd.as_ptr(), addr as *mut u8, initrd.len());
        }
        (addr, initrd.len())
    } else {
        (0, 0)
    };

    // Setup boot_params structure at 0x90000
    let boot_params = BOOT_PARAMS_ADDR as *mut u8;
    unsafe {
        ptr::write_bytes(boot_params, 0, 4096);
        // Copy first 0x1f1 bytes from kernel image
        ptr::copy_nonoverlapping(kernel_data.as_ptr(), boot_params, 0x1F1);
    }

    // Add initrd and cmdline pointers to setup header
    let setup_hdr = unsafe { &mut *(boot_params.add(0x1F1) as *mut SetupHeader) };
    setup_hdr.ramdisk_image = initrd_addr as u32;
    setup_hdr.ramdisk_size = initrd_size as u32;

    if let Some(cmdline_str) = cmdline {
        let bytes = cmdline_str.as_bytes();
        let cmdline_pages = (bytes.len() + 1 + 0xFFF) / 0x1000;
        let cmdline_addr = st.boot_services().allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            cmdline_pages,
        )?;
        unsafe {
            let dst = cmdline_addr as *mut u8;
            ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
            *dst.add(bytes.len()) = 0;
        }
        setup_hdr.cmd_line_ptr = cmdline_addr as u32;
    }

    // Exit boot services
    let (_st, _map_key) = st.exit_boot_services();

    // Jump to kernel entry
    let kernel_entry = KERNEL_LOAD_ADDR + 0x200; // Entry point offset for bzImage
    let entry_fn: extern "C" fn() -> ! = unsafe { core::mem::transmute(kernel_entry as *const ()) };
    entry_fn()
}

/// Helper to read a file into a Vec<u8>
fn read_file_to_vec(st: &SystemTable<Boot>, path: &str) -> Result<Vec<u8>> {
    let image = st.boot_services().handle_protocol::<uefi::proto::loaded_image::LoadedImage>(
        st.boot_services().image_handle())?
        .interface;
    let device = unsafe { (*image.get()).device().unwrap() };
    let fs = st.boot_services().handle_protocol::<SimpleFileSystem>(device)?.interface;
    let mut root = unsafe { (*fs.get()).open_volume()? };

    // Convert path to UTF-16
    let mut utf16 = [0u16; 256];
    let mut i = 0;
    for ch in path.encode_utf16() {
        if i >= 255 { break; }
        utf16[i] = ch;
        i += 1;
    }
    utf16[i] = 0;

    let handle = root.open(unsafe { CStr16::from_u16_with_nul_unchecked(&utf16[..=i]) }, FileMode::Read, Default::default())?;
    let mut file = match handle.into_type()? {
        FileType::Regular(f) => f,
        _ => panic!("{} is not a regular file", path),
    };

    let mut buf = Vec::new();
    loop {
        let mut chunk = [0u8; 4096];
        let read = file.read(&mut chunk)?;
        if read == 0 { break; }
        buf.extend_from_slice(&chunk[..read]);
    }
    Ok(buf)
}
*/*/