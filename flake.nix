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
        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };

        starlight = rustPlatform.buildRustPackage {
          pname = "starlight";
          version = (fromTOML (builtins.readFile ./Cargo.toml)).package.version;

          src = ./.;

          # Vendored with fetchCargoVendor, which pulls crates from static.crates.io;
          # the crates.io API endpoint rejects curl's default user agent.
          cargoHash = "sha256-4Ok9oUgrBBxpZsvvwasY6Z4C7qDrNVQdrQwdMYL7faU=";

          nativeBuildInputs =
            [ pkgs.makeWrapper ]
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.pkg-config ];

          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux linuxLibs;

          preBuild = ''
            ln -sfn gpui-component-assets-0.5.1 "$NIX_BUILD_TOP/$(stripHash "$cargoDeps")/assets"
          '';

          doCheck = false;

          postFixup = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            wrapProgram $out/bin/Starlight \
              --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath linuxLibs}
          '';

          meta = with pkgs.lib; {
            description = "Among Us mod manager";
            license = licenses.gpl3Only;
            platforms = platforms.unix;
            mainProgram = "Starlight";
          };
        };
      in
      {
        packages = {
          default = starlight;
          starlight = starlight;
        };

        apps.default = {
          type = "app";
          program = "${starlight}/bin/Starlight";
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [ toolchain ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.pkg-config ];

          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux linuxLibs;

          LD_LIBRARY_PATH = pkgs.lib.optionalString pkgs.stdenv.isLinux (pkgs.lib.makeLibraryPath linuxLibs);
        };
      }
    );
}
