{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, ... }: {
    overlays.default = final: prev: {
      lobster-rs = final.callPackage ./default.nix {};
    };
  } //
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [self.overlays.default]; };
      in
      {
        packages = {
          inherit (pkgs) lobster-rs;
          default = pkgs.lobster-rs;
        };
        devShell = with pkgs; mkShell {
          name = "lobster-rs";
          nativeBuildInputs = [ cargo rustc clippy rustfmt openssl mpv fzf pkg-config ];
        };
      }
    );
}
