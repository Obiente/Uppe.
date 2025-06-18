{
  description = "Custom flake for PeerUp's development";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils, fenix, ... }@inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        crane = inputs.crane.mkLib pkgs;
        toolchainToml = ./rust-toolchain.toml;

        toolchain = with fenix.packages.${system};
          fromToolchainFile {
            file = toolchainToml;
            sha256 = "sha256-SISBvV1h7Ajhs8g0pNezC1/KGA0hnXnApQ/5//STUbs=";
          };

        craneLib = crane.overrideToolchain toolchain;
      in {
        devShells.default = craneLib.devShell {
          packages = with pkgs; [ toolchain nodejs_22 nodePackages.pnpm ];

          env = {
            LAZYVIM_RUST_DIAGNOSTICS = "bacon-ls"; # Chiko's nvim config thing
          };
        };
      });
}
