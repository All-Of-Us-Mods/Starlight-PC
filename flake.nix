{
  description = "Starlight PC";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        toolchain = fenix.packages.${system}.stable.withComponents [
          "cargo"
          "rustc"
          "rustfmt"
          "clippy"
          "rust-src"
        ];

        linuxLibs = with pkgs; [
          bzip2
          fontconfig
          freetype
          vulkan-loader
          wayland
          libxkbcommon
          libGL
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libxcb
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs =
            [ toolchain ]
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.pkg-config ];

          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux linuxLibs;

          LD_LIBRARY_PATH =
            pkgs.lib.optionalString pkgs.stdenv.isLinux (pkgs.lib.makeLibraryPath linuxLibs);
        };
      }
    );
}
