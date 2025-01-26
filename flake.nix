{
  description = "A CLI tool to watch movies and TV shows";
  
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          buildInputs = with pkgs; [
            openssl
            pkg-config
            mpv
            fzf
            rofi
            ffmpeg
            chafa
          ];
          nativeBuildInputs = with pkgs; [
            openssl.dev
            pkg-config
            makeWrapper
          ];
          LD_LIBRARY_PATH = "${pkgs.openssl.out}/lib";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        lobster-rs = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          postInstall = ''
            wrapProgram $out/bin/lobster-rs \
              --prefix PATH : ${pkgs.lib.makeBinPath [
                pkgs.mpv
                pkgs.fzf
                pkgs.rofi
                pkgs.ffmpeg
                pkgs.chafa
              ]}
          '';
        });
      in {
        packages = {
          default = lobster-rs;
        };
        
        apps.default = flake-utils.lib.mkApp {
          drv = lobster-rs;
        };
        
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            cargo
            rustc
            rust-analyzer
            rustfmt
            clippy
            openssl
            pkg-config
            openssl.dev
            mpv
            fzf
            rofi
            ffmpeg
            chafa
          ];
        };
        
        formatter = pkgs.nixpkgs-fmt;
      });
}
