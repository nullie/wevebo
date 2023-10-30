{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };
    in {
      packages.hound = pkgs.rustPlatform.buildRustPackage {
        name = "hound";

        src = ./.;

        cargoLock.lockFile = ./Cargo.lock;

        buildInputs = [pkgs.pkg-config pkgs.openssl.dev];
      };

      defaultPackage = self.packages.${system}.hound;

      devShell = pkgs.mkShell {
        inputsFrom = builtins.attrValues self.packages.${system};
      };

      formatter = pkgs.alejandra;
    });
}
