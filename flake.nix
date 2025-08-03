{
  description = "Android NDK FHS Shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.default = let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
      };
    in
      pkgs.buildFHSEnv {
        name = "ndk-clang-env";
        targetPkgs = pkgs: [
          pkgs.gcc
          pkgs.glibc
          pkgs.zlib
          pkgs.coreutils
        ];
        runScript = "zsh";
      };
  };
}
