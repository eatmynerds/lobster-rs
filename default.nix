{ rustPlatform
, lib
, openssl
, pkg-config
,
}:
let manifest = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name;
  version = manifest.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = lib.cleanSource ./.;

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];

  doCheck = false;

  meta = {
    description = "A CLI tool to watch movies and TV shows";
    homepage = "https://github.com/eatmynerds/lobster-rs";
    license = lib.licenses.mit;
    mainProgram = "lobster-rs";
    platforms = lib.platforms.unix;
  };
}
