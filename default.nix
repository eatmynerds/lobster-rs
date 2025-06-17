{ lib, rustPlatform, nix-gitignore }: 
let 
  version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
  src = nix-gitignore.gitignoreSource [] ./.;
in rustPlatform.buildRustPackage {
    pname = "lobster-rs";
    inherit src version;
    cargoLock.lockFile = ./Cargo.lock;
    doCheck = false;
    meta = {
      description ="Terminal file deletion, reanimated";
      license = lib.licenses.mit;
      platforms = lib.platforms.linux;
      mainProgram = "lobster-rs";
    };
}
