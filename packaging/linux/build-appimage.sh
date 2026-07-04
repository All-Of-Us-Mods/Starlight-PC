#!/usr/bin/env bash
# Build Starlight-linux-x86_64.AppImage from the release binary. Run from the
# repo root after `cargo build --release`. Downloads appimagetool on demand.
set -euo pipefail

BINARY=target/release/Starlight
OUT=Starlight-linux-x86_64.AppImage
APPDIR=target/appimage/AppDir

[ -f "$BINARY" ] || { echo "error: $BINARY not found — run cargo build --release first" >&2; exit 1; }

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps" \
         "$APPDIR/usr/share/metainfo"

cp "$BINARY" "$APPDIR/usr/bin/starlight"
cp packaging/linux/dev.allofus.Starlight.desktop "$APPDIR/usr/share/applications/"
cp packaging/linux/dev.allofus.Starlight.metainfo.xml "$APPDIR/usr/share/metainfo/"
cp assets/icons/starlight.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/dev.allofus.Starlight.png"

# Top-level files appimagetool requires.
cp packaging/linux/dev.allofus.Starlight.desktop "$APPDIR/"
cp assets/icons/starlight.png "$APPDIR/dev.allofus.Starlight.png"
ln -sf dev.allofus.Starlight.png "$APPDIR/.DirIcon"
cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/starlight" "$@"
EOF
chmod +x "$APPDIR/AppRun"

TOOL=target/appimage/appimagetool
if [ ! -x "$TOOL" ]; then
    curl -fsSL -o "$TOOL" \
        https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$TOOL"
fi

# --appimage-extract-and-run avoids needing FUSE on CI runners.
ARCH=x86_64 "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT"
echo "built $OUT"
