{
  description = "Oneil -  Design specification language for rapid, comprehensive system modeling.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs: inputs.flake-parts.lib.mkFlake { inherit inputs; } {
    systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
    perSystem = { pkgs, ... }: {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          # Rust tools
          rustc
          cargo
          clippy
          rustfmt
          rust-analyzer

          # VSCode extension tools
          nodejs_20
          pnpm
          vsce # "Visual Studio Code Extension Manager"
        ];
      };

      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "oneil";
        version = "0.16.0";
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };
      };
    };
  };
}
