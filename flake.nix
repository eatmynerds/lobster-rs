{
	description = "wall-utils, a simple wallpaper utility to easily switch and select wallpapers";
	inputs = {
		nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
		utils.url = "github:numtide/flake-utils";
	};

	outputs = { self, nixpkgs, utils, ... }: {
		overlays.default = final: prev: {
			lobster-rs = final.callPackage ./build.nix {};
		};
	}
	// 
	utils.lib.eachDefaultSystem (system:
		let pkgs = import nixpkgs {
			inherit system;
			overlays = [self.overlays.default];
		};
		in {
			packages = {
				inherit (pkgs) lobster-rs;
				default = pkgs.lobster-rs;
			};

			devShells.default = pkgs.mkShell {
				name = "lobster-rs";
        buildInputs = with pkgs; [ cargo rustc rustfmt pkg-config openssl ]; 
        nativeBuildInputs = with pkgs; [ openssl.dev ]; 
        			};
		}
	);
}
