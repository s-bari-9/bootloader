#!/usr/bin/env bash
set -e

TARGET_EFI="$1"
ESP_DIR="esp/EFI/BOOT"
#ESP_IMG="esp.img"

# Create the directory for the ESP
#mkdir -p "$ESP_DIR"
rm -f "$ESP_DIR/BOOTX64.EFI" "$ESP_DIR/KERNEL.EFI"
cp "$TARGET_EFI" "$ESP_DIR/BOOTX64.EFI" 
cargo build --bin kernel
cp $(dirname "$TARGET_EFI")/kernel.efi "$ESP_DIR/KERNEL.EFI"

# Create a 64MB FAT-formatted disk image
#dd if=/dev/zero of="$ESP_IMG" bs=1M count=64 status=none
#mkfs.vfat -n 'ESP' "$ESP_IMG" >/dev/null

# Mount and copy ESP contents
#mcopy -i "$ESP_IMG" -s esp/* ::/

# Run with QEMU + OVMF
#qemu-system-x86_64 \
#  -bios /usr/share/OVMF/OVMF.fd \
#  -drive file="$ESP_IMG",format=raw,if=virtio \
#  -nographic

#qemu-system-x86_64 \
#  -m 512M \
#  -drive if=pflash,format=raw,file=extra/OVMF.fd \
#  -drive file=fat:rw:esp,format=raw
qemu-system-x86_64 \
  -m 1G \
  -smp 2 \
  -accel kvm \
  -cpu host \
  -drive if=pflash,format=raw,file=extra/OVMF.fd \
  -drive if=virtio,format=qcow2,file=/var/lib/libvirt/images/archlinux-2.qcow2 \
  -drive file=fat:rw:esp,format=raw,index=0,media=disk
# Expose the host directory as a FAT drive
#  -no-reboot \                                  # Prevents QEMU from rebooting on exit
#  -nographic   
#  -enable-kvm \                                 # Use KVM for better performance (if available)
#  -m 512M \                                     # Allocate some RAM for the VM
#  -cpu host \                                   # Use host CPU features
#  -drive if=pflash,format=raw,readonly=on,file=/usr/share/ovmf/OVMF_CODE.fd \ # OVMF firmware code