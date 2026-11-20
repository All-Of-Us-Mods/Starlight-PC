{
  description = "Starlight PC";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
        toolchain = fenix.packages.${system}.default.toolchain;

        linuxLibs = with pkgs; [
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

        starlight =
          (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          }).buildRustPackage
            {
              pname = "starlight";
              version = (fromTOML (builtins.readFile ./Cargo.toml)).package.version;
              src = ./.;

              cargoLock.lockFile = ./Cargo.lock;
              doCheck = false;

              nativeBuildInputs = [ pkgs.makeWrapper ] ++ pkgs.lib.optional pkgs.stdenv.isLinux pkgs.pkg-config;
              buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux linuxLibs;

              postFixup = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
                wrapProgram $out/bin/Starlight \
                  --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath linuxLibs}
              '';

              meta = {
                description = "Among Us mod manager";
                license = pkgs.lib.licenses.gpl3Only;
                platforms = pkgs.lib.platforms.unix;
                mainProgram = "Starlight";
              };
            };
      in
      {
        packages = {
          default = starlight;
          inherit starlight;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ starlight ];
          packages = [ toolchain ];
          LD_LIBRARY_PATH = pkgs.lib.optionalString pkgs.stdenv.isLinux (pkgs.lib.makeLibraryPath linuxLibs);
        };
      }
    );
}
