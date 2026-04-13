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
            pkg-config
            openssl
            glib
            gtk3
            webkitgtk_4_1
            libsoup_3
            glib-networking
            cacert
            cairo
            pango
            gdk-pixbuf
            atk
            rustc
            cargo
            rustfmt
            clippy
          ];

          shellHook = ''
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules/"
            export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules/"
            export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"

            echo "Bun + Rust dev shell ready"
            bun --version
            node --version
            cargo --version
            pkg-config --version
          '';
        };
      }
    );
}
