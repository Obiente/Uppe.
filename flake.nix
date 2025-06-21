{
  description = "Flake for PeerUp's development";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      fenix,
      crane,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        defaultResult = import ./default.nix {
          inherit
            nixpkgs
            system
            crane
            fenix
            ;
        };
      in
      {
        devShells = {
          default = defaultResult.sharedShell;

          node = defaultResult.nodeShell;
          rust = defaultResult.rustShell;
        };

        packages = {
          inherit (defaultResult) client;
          inherit (defaultResult) service;
        };
      }
    );
}
