{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
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
      packages.wevebo = pkgs.rustPlatform.buildRustPackage {
        name = "wevebo";

        src = ./.;

        cargoLock.lockFile = ./Cargo.lock;

        buildInputs = [pkgs.openssl];
        nativeBuildInputs = [pkgs.pkg-config];
      };

      defaultPackage = self.packages.${system}.wevebo;

      devShell = pkgs.mkShell {
        packages = [pkgs.rustfmt pkgs.clippy pkgs.cargo-edit];
        inputsFrom = builtins.attrValues self.packages.${system};
      };

      formatter = pkgs.alejandra;
    });
}
