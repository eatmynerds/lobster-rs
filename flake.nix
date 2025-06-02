{
  description = "A CLI tool to watch movies and TV shows";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      eachSystem = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = eachSystem (system:
        import nixpkgs {
          config = { };
          localSystem = system;
          overlays = [ ];
        });
    in
    {
      packages = eachSystem (system: {
        lobster-rs = pkgsFor.${system}.callPackage ./default.nix { };
        default = self.packages.${system}.lobster-rs;
      });
    };
}
