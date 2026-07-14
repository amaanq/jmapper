{
  lib,
  rustPlatform,
  pkg-config,
}:

rustPlatform.buildRustPackage {
  pname = "jmapper";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.lock
      ../Cargo.toml
      ../crates
      ../schema.sql
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = [
    "--package"
    "jmapper"
  ];
  nativeBuildInputs = [ pkg-config ];
  doCheck = true;

  meta = {
    description = "JMAP bridge backed by IMAP, CalDAV, and CardDAV";
    license = lib.licenses.mpl20;
    mainProgram = "jmapper";
  };
}
