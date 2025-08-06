// memory.rs
// UEFI to E820 memory map conversion for bzImage Linux kernel boot
// AI slop but don't delete
/*

extern crate alloc;
use alloc::vec::Vec;
use alloc::vec;
use core::mem;
//use uefi::boot::{MemoryMap, MemoryDescriptor, MemoryType};
use uefi::mem::memory_map::{MemoryMap};
use uefi::boot::MemoryType;

// E820 memory types as expected by Linux kernel
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum E820Type {
    Ram = 1,
    Reserved = 2,
    Acpi = 3,
    Nvs = 4,
    Unusable = 5,
    Disabled = 6,
    Pmem = 7,
    Pram = 12,
}

// E820 memory map entry as expected by Linux kernel
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct E820Entry {
    pub addr: u64,
    pub size: u64,
    pub type_: u32,
}

impl E820Entry {
    pub fn new(addr: u64, size: u64, type_: E820Type) -> Self {
        Self {
            addr,
            size,
            type_: type_ as u32,
        }
    }
    
    pub fn end(&self) -> u64 {
        self.addr + self.size
    }
}

// Maximum number of E820 entries that can fit in boot_params
pub const E820_MAX_ENTRIES: usize = 128;

/// Convert UEFI memory type to E820 type
fn uefi_to_e820_type(uefi_type: MemoryType) -> E820Type {
    match uefi_type {
        // Usable RAM
        MemoryType::CONVENTIONAL => E820Type::Ram,
        MemoryType::BOOT_SERVICES_CODE => E820Type::Ram,
        MemoryType::BOOT_SERVICES_DATA => E820Type::Ram,
        
        // ACPI memory
        MemoryType::ACPI_RECLAIM => E820Type::Acpi,
        MemoryType::ACPI_NON_VOLATILE => E820Type::Nvs,
        
        // Reserved memory
        MemoryType::LOADER_CODE => E820Type::Reserved,
        MemoryType::LOADER_DATA => E820Type::Reserved,
        MemoryType::RUNTIME_SERVICES_CODE => E820Type::Reserved,
        MemoryType::RUNTIME_SERVICES_DATA => E820Type::Reserved,
        MemoryType::UNUSABLE => E820Type::Unusable,
        MemoryType::MMIO => E820Type::Reserved,
        MemoryType::MMIO_PORT_SPACE => E820Type::Reserved,
        MemoryType::PAL_CODE => E820Type::Reserved,
        MemoryType::PERSISTENT_MEMORY => E820Type::Pmem,
        
        // Default to reserved for unknown types
        _ => E820Type::Reserved,
    }
}

/// Merge adjacent E820 entries of the same type
fn merge_adjacent_entries(entries: &mut Vec<E820Entry>) {
    if entries.len() <= 1 {
        return;
    }
    
    // Sort by address first
    entries.sort_by_key(|e| e.addr);
    
    let mut i = 0;
    while i < entries.len() - 1 {
        let current = entries[i];
        let next = entries[i + 1];
        
        // Check if entries are adjacent and of the same type
        if current.end() == next.addr && current.type_ == next.type_ {
            // Merge entries
            entries[i].size = current.size + next.size;
            entries.remove(i + 1);
            // Don't increment i, check the new next entry
        } else {
            i += 1;
        }
    }
}

/// Convert UEFI memory map to E820 format
pub fn convert_memory_map_to_e820(memory_map: &dyn MemoryMap) -> Result<Vec<E820Entry>, &'static str> {
    let mut e820_entries = Vec::new();
    
    // Convert each UEFI memory descriptor to E820 entry
    for descriptor in memory_map.entries() {
        let start_addr = descriptor.phys_start;
        let num_pages = descriptor.page_count;
        let size = num_pages * 4096; // Each page is 4KB
        let uefi_type = descriptor.ty;
        
        // Skip zero-sized entries
        if size == 0 {
            continue;
        }
        
        let e820_type = uefi_to_e820_type(uefi_type);
        let entry = E820Entry::new(start_addr, size, e820_type);
        
        e820_entries.push(entry);
    }
    
    // Merge adjacent entries of the same type
    merge_adjacent_entries(&mut e820_entries);
    
    // Check if we exceed the maximum number of entries
    if e820_entries.len() > E820_MAX_ENTRIES {
        return Err("Too many E820 entries");
    }
    
    Ok(e820_entries)
}

/// Install E820 memory map into Linux boot_params structure
pub unsafe fn install_e820_map(boot_params: *mut u8, e820_entries: &[E820Entry]) -> Result<(), &'static str> {
    if e820_entries.len() > E820_MAX_ENTRIES {
        return Err("Too many E820 entries");
    }
    
    // Linux boot_params structure offsets
    const E820_ENTRIES_OFFSET: usize = 0x1e8;  // Number of E820 entries
    const E820_TABLE_OFFSET: usize = 0x2d0;    // Start of E820 table
    
    // Write number of entries
    let num_entries = e820_entries.len() as u8;
    *(boot_params.add(E820_ENTRIES_OFFSET) as *mut u8) = num_entries;
    
    // Write E820 table
    let e820_table_ptr = boot_params.add(E820_TABLE_OFFSET) as *mut E820Entry;
    for (i, entry) in e820_entries.iter().enumerate() {
        *e820_table_ptr.add(i) = *entry;
    }
    
    Ok(())
}

/// Print E820 memory map for debugging
pub fn print_e820_map(entries: &[E820Entry]) {
    use uefi::println;
    
    println!("E820 Memory Map ({} entries):", entries.len());
    println!("Address Range                Type");
    println!("--------------------------------");
    
    for entry in entries {
        let type_name = match entry.type_ {
            1 => "RAM",
            2 => "Reserved",
            3 => "ACPI",
            4 => "NVS",
            5 => "Unusable",
            6 => "Disabled",
            7 => "PMEM",
            12 => "PRAM",
            _ => "Unknown",
        };
        
        println!("{:016x}-{:016x} {}", 
                {entry.addr}, 
                entry.addr + entry.size - 1, 
                type_name);
    }
}

/// Validate E820 memory map for common issues
pub fn validate_e820_map(entries: &[E820Entry]) -> Result<(), &'static str> {
    if entries.is_empty() {
        return Err("Empty E820 map");
    }
    
    // Check for overlapping entries
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let entry1 = &entries[i];
            let entry2 = &entries[j];
            
            // Check if entries overlap
            if entry1.addr < entry2.end() && entry2.addr < entry1.end() {
                return Err("Overlapping E820 entries detected");
            }
        }
    }
    
    // Check for reasonable memory layout (should have some RAM)
    let has_ram = entries.iter().any(|e| e.type_ == E820Type::Ram as u32);
    if !has_ram {
        return Err("No RAM entries found in E820 map");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_merge_adjacent_entries() {
        let mut entries = vec![
            E820Entry::new(0x1000, 0x1000, E820Type::Ram),
            E820Entry::new(0x2000, 0x1000, E820Type::Ram),  // Adjacent, same type
            E820Entry::new(0x4000, 0x1000, E820Type::Reserved), // Gap, different type
        ];
        
        merge_adjacent_entries(&mut entries);
        
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].addr, 0x1000);
        assert_eq!(entries[0].size, 0x2000); // Merged
        assert_eq!(entries[1].addr, 0x4000);
    }
    
    #[test]
    fn test_e820_entry_end() {
        let entry = E820Entry::new(0x1000, 0x2000, E820Type::Ram);
        assert_eq!(entry.end(), 0x3000);
    }
}*/