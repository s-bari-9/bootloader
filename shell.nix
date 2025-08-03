{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [ pkgs.qemu pkgs.OVMF.fd ];
  shellHook = ''
    export OVMF=$(nix eval --raw nixpkgs#OVMF.fd)
    echo "OVMF UEFI firmware path: $OVMF"
  '';
}

