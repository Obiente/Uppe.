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

  nodePackages = [
    versionedNode
    pkgs.astro-language-server
    pkgs.nodePackages.serve
    pkgs.pnpm_10
  ];
  rustPackages = [ fenixToolchain ];

  inherit (pkgs) lib mkShellNoCC buildNpmPackage;
in
{
  nodeShell = mkShellNoCC {
    name = "node-client";
    packages = nodePackages;
  };

  rustShell = craneWithTC.devShell {
    name = "rust-service";
    packages = rustPackages;
  };

  sharedShell = craneWithTC.devShell {
    name = "shared-shell";
    packages = nodePackages ++ rustPackages;
  };

  client = buildNpmPackage {
    pname = "PeerUP-client";
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
  };
}
