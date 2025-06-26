{
  lib,
  rustPlatform,
  name,
  version,
}:
(rustPlatform.buildRustPackage {
  pname = name;
  inherit version;
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
    allowBuiltinFetchGit = true;
  };
  meta = with lib; {
    homepage = "https://github.com/O-X-E-Y/oxeylyzer";
    license = licenses.asl20;
    mainProgram = "oxeylyzer";
  };
})
