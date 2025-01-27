{
  description = "Add a description to me!";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages = {
          lobsterRS = pkgs.rustPlatform.buildRustPackage {
            pname = "lobster-rs";
            version = "0.1.0";
            src = ./.;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            shellHook = ''
              export PKG_CONFIG_PATH=${pkgs.openssl.dev}/lib/pkgconfig
            '';
          };
        };

        defaultPackage = self.packages.${system}.lobsterRS;
      });
}

