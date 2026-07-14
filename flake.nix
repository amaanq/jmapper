{
  description = "JMAP proxy for IMAP, CalDAV, and CardDAV accounts";

  outputs =
    { self, ... }@args:
    let
      inputs = (import ./.tack) { overrides = args.tackOverrides or { }; };
      inherit (inputs) fenix nixpkgs;
      inherit (nixpkgs) lib;
      forAllSystems = lib.genAttrs (lib.systems.doubles.linux ++ lib.systems.doubles.darwin);
      pkgsFor = system: nixpkgs.legacyPackages.${system} or (import nixpkgs { inherit system; });

      hasFenix = system: fenix.packages ? ${system};
      rustPlatformFor =
        system:
        let
          pkgs = pkgsFor system;
          fenixPkgs = fenix.packages.${system}.latest;
        in
        if hasFenix system then
          pkgs.makeRustPlatform { inherit (fenixPkgs) cargo rustc; }
        else
          pkgs.rustPlatform;
      toolchainFor =
        system:
        let
          pkgs = pkgsFor system;
        in
        if hasFenix system then
          with fenix.packages.${system}.latest;
          [
            cargo
            clippy
            rust-src
            rustc
            rustfmt
          ]
        else
          (with pkgs; [
            cargo
            clippy
            rustc
            rustfmt
          ]);
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = self.packages.${system}.jmapper;
          jmapper = pkgs.callPackage ./nix/package.nix {
            rustPlatform = rustPlatformFor system;
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.jmapper ];
            packages = toolchainFor system ++ [
              pkgs.cargo-deny
              pkgs.cornucopia
              pkgs.postgresql_16
              pkgs.rust-analyzer
              pkgs.taplo
            ];
            shellHook = ''
              : "''${JMAPPER_TEST_DB_URL:=host=/tmp dbname=jmapper_test}"
              export JMAPPER_TEST_DB_URL
            '';
          };
        }
      );

      checks = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          jmapper = self.packages.${system}.jmapper;
          clippy = if hasFenix system then fenix.packages.${system}.latest.clippy else pkgs.clippy;
          fmtSrc = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./.rustfmt.toml
              ./.tack
              ./Cargo.lock
              ./Cargo.toml
              ./crates
              ./flake.nix
              ./nix
            ];
          };
        in
        {
          default = pkgs.linkFarmFromDrvs "jmapper-checks" [
            jmapper
            self.checks.${system}.fmt
            self.checks.${system}.clippy
          ];

          inherit jmapper;
          fmt =
            pkgs.runCommand "jmapper-fmt-check"
              {
                nativeBuildInputs =
                  (
                    if hasFenix system then
                      with fenix.packages.${system}.latest;
                      [
                        cargo
                        rustfmt
                      ]
                    else
                      [
                        pkgs.cargo
                        pkgs.rustfmt
                      ]
                  )
                  ++ [ pkgs.nixfmt ];
                src = fmtSrc;
              }
              ''
                cp -r $src ./tree
                chmod -R +w ./tree
                cd ./tree
                cargo fmt --all -- --check
                find . -name '*.nix' -exec nixfmt --check {} +
                touch $out
              '';
          clippy = jmapper.overrideAttrs (old: {
            pname = "jmapper-clippy";
            nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ clippy ];
            buildPhase = ''
              runHook preBuild
              cargo clippy --workspace --all-targets --all-features --offline --locked -- -D warnings
              runHook postBuild
            '';
            checkPhase = "true";
            doCheck = false;
            installPhase = ''
              runHook preInstall
              touch $out
              runHook postInstall
            '';
          });
        }
      );

      nixosModules = {
        default = import ./nix/module.nix self;
        jmapper = import ./nix/module.nix self;
      };

      formatter = forAllSystems (system: (pkgsFor system).nixfmt);
    };
}
