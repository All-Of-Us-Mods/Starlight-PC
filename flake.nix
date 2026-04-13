{
  description = "Dev shell with Bun and Rust for Starlight PC";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            bun
            nodejs_24
            rustc
            cargo
            rustfmt
            clippy
          ];

          shellHook = ''
            echo "Bun + Rust dev shell ready"
            bun --version
            node --version
            cargo --version
          '';
        };
      }
    );
}
