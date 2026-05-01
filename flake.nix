{
  description = "Starlight PC";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        pname = "starlight";
        version = "1.1.1";

        src = lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            let
              rel = lib.removePrefix (toString ./. + "/") (toString path);
              base = baseNameOf path;
            in
            !(lib.hasPrefix ".git/" rel)
            && !(lib.hasPrefix "build/" rel)
            && !(lib.hasPrefix "node_modules/" rel)
            && !(lib.hasPrefix "src-tauri/target/" rel)
            && base != "result";
        };

        bunDeps = pkgs.stdenvNoCC.mkDerivation {
          pname = "${pname}-bun-deps";
          inherit version src;

          nativeBuildInputs = [ pkgs.bun ];

          dontConfigure = true;
          dontBuild = true;

          installPhase = ''
            runHook preInstall

            export HOME="$TMPDIR"
            bun install --frozen-lockfile --ignore-scripts

            mkdir -p "$out"
            cp -R node_modules "$out/"

            runHook postInstall
          '';

          outputHashAlgo = "sha256";
          outputHashMode = "recursive";
          outputHash = "sha256-81JoWZNK5PYYmsIOWb7KLZyw7P3Y2B3czuye2b5XxGM=";
        };
      in
      {
        packages.default = pkgs.stdenv.mkDerivation {
          inherit pname version src;

          cargoDeps = pkgs.rustPlatform.importCargoLock {
            lockFile = ./src-tauri/Cargo.lock;
          };
          cargoRoot = "src-tauri";

          nativeBuildInputs = with pkgs; [
            bun
            cargo
            cargo-tauri
            nodejs
            pkg-config
            rustPlatform.cargoSetupHook
            rustc
            wrapGAppsHook4
          ];

          buildInputs = with pkgs; [
            glib-networking
            librsvg
            openssl
            webkitgtk_4_1
          ];

          env = {
            OPENSSL_NO_VENDOR = "1";
            PUBLIC_API_URL = "https://starlight.allofus.dev";
          };

          configurePhase = ''
            runHook preConfigure

            export HOME="$TMPDIR"

            cp -R ${bunDeps}/node_modules ./node_modules
            chmod -R u+w ./node_modules
            patchShebangs ./node_modules

            export PATH="$PWD/node_modules/.bin:$PATH"

            runHook postConfigure
          '';

          buildPhase = ''
            runHook preBuild

            cargo tauri build --no-bundle

            runHook postBuild
          '';

          installPhase = ''
                runHook preInstall

                bin="$(find src-tauri/target/release -maxdepth 1 \( -name 'Starlight' -o -name 'starlight' \) -type f -perm -0100 | head -n1)"

                if [ -z "$bin" ]; then
                  echo "could not find built starlight binary" >&2
                  find src-tauri/target/release -maxdepth 2 -type f >&2
                  exit 1
                fi

                install -Dm755 "$bin" "$out/bin/starlight"
                install -Dm644 src-tauri/icons/128x128.png "$out/share/icons/hicolor/128x128/apps/starlight.png"
                install -Dm644 static/starlight.png "$out/share/pixmaps/starlight.png"

                install -Dm644 /dev/stdin "$out/share/applications/starlight.desktop" <<EOF
            [Desktop Entry]
            Type=Application
            Name=Starlight
            Comment=Among Us mod manager
            Exec=starlight
            Icon=starlight
            Categories=Game;
            Terminal=false
            StartupWMClass=Starlight
            EOF

                runHook postInstall
          '';

          meta = {
            description = "Among Us mod manager";
            homepage = "https://github.com/All-Of-Us-Mods/Starlight-PC";
            license = lib.licenses.gpl3Only;
            mainProgram = "starlight";
            platforms = lib.platforms.linux;
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            openssl
            wrapGAppsHook4
            cargo
            nodejs
            bun
            rustc # Needed for dev server (npm tauri dev)
            rustfmt
            clippy
          ];

          buildInputs = with pkgs; [
            librsvg
            webkitgtk_4_1
          ];

          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
            export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH" # Needed on Wayland to report the correct display scale
            export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            export NIX_SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
          '';
        };
      }
    );
}
