{
  description = "A single tool for opening groups of applications, files, folders, and URLs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        systems = [
          "x86_64-darwin"
          "aarch64-darwin"
        ];
        forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
      in
      {
        packages = forAllSystems (
          system:
          let
            pkgs = import nixpkgs { inherit system; };
            manifest = (pkgs.lib.importTOML ./Cargo.toml).package;

            # Map each system to its GitHub release asset
            srcs = {
              x86_64-darwin = pkgs.fetchurl {
                url = "${manifest.repository}/releases/download/v${manifest.version}/${manifest.name}-${manifest.version}-x86_64-apple-darwin.tar.gz";
                sha256 = "sha256-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"; # fill with nix store prefetch-file
              };
              aarch64-darwin = pkgs.fetchurl {
                url = "${manifest.repository}/releases/download/v${manifest.version}/${manifest.name}-${manifest.version}-aarch64-apple-darwin.tar.gz";
                sha256 = "sha256-yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy"; # fill with nix store prefetch-file
              };
            };
          in
          pkgs.stdenv.mkDerivation {
            pname = "${manifest.name}";
            version = "${manifest.version}";

            # Select the right tarball
            src = pkgs.lib.attrByPath [ system ] (throw "Unsupported platform ${system}") srcs;

            # Do not auto-unpack
            unpackPhase = "true";

            installPhase = ''
              mkdir -p $out/bin
              tar -xzf $src -C $out/bin
            '';

            meta = {
              description = "${manifest.description}";
              homepage = "${manifest.repository}";
              license = pkgs.lib.licenses.mit;
              platforms = builtins.attrNames srcs;
            };
          }
        );
      }
    );
}
