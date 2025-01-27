{ pkgs, lib, rustPlatform, bash, makeWrapper, pkg-config }: 
let 
	version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
	src = ./.;
in 
rustPlatform.buildRustPackage rec {
	pname = "lobster-rs";
	inherit src version;
	cargoLock.lockFile = ./Cargo.lock;
shellHook = '' 
        export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig" 
        '';

	nativeBuildInputs = [
		makeWrapper
		pkg-config
	];
	buildInputs = [
		bash
	];
	doCheck = false;

	fixupPhase = ''
		wrapProgram $out/bin/${pname} --set PATH ${bash}/bin:\$PATH
	'';

	meta = {
		description = "Efficient wallpaper switching utiltiy";
		license = lib.licenses.gpl3;
		platforms = lib.platforms.linux;
		mainProgram = "lobster-rs";
	};
}

