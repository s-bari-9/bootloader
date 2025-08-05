# Rust UEFI Bootloader

A UEFI bootloader (systemd-boot compatible(not yet!))

## Directory Structure

Expected layout on the EFI System Partition (XBOOTLDR partition is **NOT** supported right now, so use a single ESP partition for now):

```
/EFI/BOOT/BOOTX64.EFI         <= This bootloader
/EFI/BOOT/KERNEL.EFI          <= Hello world file for testing bootloader
/loader/loader.conf           <= Global config (TODO)
/loader/entries/*.conf       <= Per-entry configs
````

Boot entries are supported as in [UAPI specifications](https://uapi-group.org/specifications/specs/boot_loader_specification/#type-1-boot-loader-specification-entries).


## TODO


* [X] Load UEFI executables
* [-] Load non-EFI Linux kernels
* [-] Support [bootloader entries](https://uapi-group.org/specifications/specs/boot_loader_specification/#type-1-boot-loader-specification-entries)
    * [x] Kernel files found in `/EFI/Linux/`
    * [x] UEFI shell `/shellx64.efi`
* [-] Boot menu selection UI
* [X] Windows chainloading
* [-] Apple chainloading (Just search in `EFI\Apple\Boot\boot.efi` for now)
* [ ] XBOOTLDR partition support
* [ ] Bootloader conf
* [-] Pass kernel options
* [-] Initrd loading
* [ ] ACPI and memory map handoff

## Extra
* [ ] Encrypted XBOOTLDR support

( [-] for partial support)

## Building

* Install rust toolchain for uefi:

```sh
rustup target add x86_64-unknown-uefi
```

* Build the UEFI executable:

```sh
cargo build
```

Here is the `.cargo/config.toml`:

```toml
[build]
target = "x86_64-unknown-uefi"

[target.x86_64-unknown-uefi]
runner = "extra/efirunner.sh"
linker = "rust-lld"
```

* Test in qemu: `efirunner.sh` does this.

## License

Licensed under GPL-3.0
