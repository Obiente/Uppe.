{
  system ? builtins.currentSystem,
  nixpkgs,
  crane,
  fenix,
}:
let
  pkgs = import nixpkgs { inherit system; };
  fenixToolchain =
    with fenix.packages.${system};
    fromToolchainFile {
      file = ./rust-toolchain.toml;
      sha256 = "sha256-hPIBpoDATIduLcMs1jk8ZLhM9fXXZUslxE0kMtzosso=";
    };
  craneWithTC = (crane.mkLib pkgs).overrideToolchain fenixToolchain;

  versionedNode = pkgs.nodejs_24;
  versionedLibCxx = pkgs.stdenv.cc.cc.lib;

  nodePackages = [
    versionedNode
    pkgs.svelte-language-server
    pkgs.astro-language-server
    pkgs.nodePackages.serve
    pkgs.pnpm_10
  ];
  rustPackages = [ fenixToolchain ];

  mkCraneShell =
    name: packages:
    craneWithTC.devShell {
      inherit name;
      inherit packages;
      buildInputs = [ versionedLibCxx ];
      LD_LIBRARY_PATH = lib.makeLibraryPath [ versionedLibCxx ];
    };

  inherit (pkgs) lib mkShellNoCC buildNpmPackage;
in
{
  nodeShell = mkShellNoCC {
    name = "node-client";
    packages = nodePackages;
  };

  rustShell = mkCraneShell "rust-service" rustPackages;
  sharedShell = mkCraneShell "shared-shell" (nodePackages ++ rustPackages);

  client = buildNpmPackage {
    pname = "Uppe-client";
    version = (lib.importJSON ./apps/client/package.json).version;
    nodejs = versionedNode;

    src = ./apps/client;
    npmDepsHash = "sha256-TVQ5ELx1bmFnqF5IN2DsAAHxNGNZLW9DrzmKav1uHLM=";

    installPhase = ''
      mkdir -p $out/srv/
      cp -r dist/* $out/srv/
    '';
  };

  service = craneWithTC.buildPackage {
    src = craneWithTC.cleanCargoSource ./apps/service;
    strictDeps = true;
    doCheck = false;

    nativeBuildInputs = [ pkgs.autoPatchelfHook ];
    buildInputs = [ versionedLibCxx ];
  };
}
