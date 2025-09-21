{
  description = "A tool to open groups of applications, files, folders, and URLs";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      pkgs = nixpkgs.legacyPackages.aarch64-darwin;
    in
    {
      packages.aarch64-darwin.kozutsumi = pkgs.rustPlatform.buildRustPackage {
        pname = "kozutsumi";
        version = "0.1.1";
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };
        platforms = [ "aarch64-darwin" ];
        rustChannel = "nightly";
      };

      defaultPackage.aarch64-darwin = self.packages.aarch64-darwin.kozutsumi;
    };
}
