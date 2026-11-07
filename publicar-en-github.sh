#!/bin/sh
# Sube Lente a GitHub (https://github.com/cotizcesar/lente, público) y publica la versión actual.
# GitHub construye entonces todos los paquetes (tarball, .deb, .rpm, AppImage, Flatpak)
# y los adjunta a la versión, con install.sh para instalar desde la terminal.
#
# Necesita git con tu clave SSH de GitHub. Si tienes gh (GitHub CLI) con sesión iniciada, crea
# el repositorio solo; si no, te pide crearlo vacío en https://github.com/new.
# Uso: sh publicar-en-github.sh
set -eu
cd "$(dirname "$0")"
REPO="cotizcesar/lente"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
command -v git >/dev/null || { echo "Falta git."; exit 1; }

[ -d .git ] || git init -q -b main
git remote get-url origin >/dev/null 2>&1 || git remote add origin "git@github.com:$REPO.git"

if ! git ls-remote origin >/dev/null 2>&1; then
    if command -v gh >/dev/null 2>&1; then
        # GitHub CLI crea el repositorio; si no hay sesión, la abre primero (código en el navegador).
        gh auth status >/dev/null 2>&1 || gh auth login --hostname github.com --git-protocol ssh --web
        gh repo create "$REPO" --public \
            --description "Control de cámaras web (Elgato Facecam y UVC) para Linux"
    else
        echo "El repositorio https://github.com/$REPO todavía no existe."
        echo "Créalo en https://github.com/new con el nombre «lente», Public y sin README, licencia"
        echo "ni .gitignore. Después pulsa Intro."
        read -r _
        git ls-remote origin >/dev/null 2>&1 || { echo "Sigue sin existir (o tu clave SSH no tiene acceso)."; exit 1; }
    fi
fi

git add -A
git commit -q -m "Lente $VERSION" || true
git tag -a "v$VERSION" -m "Lente $VERSION" 2>/dev/null || true
git push -u origin main
git push origin "v$VERSION"
if command -v gh >/dev/null 2>&1; then
    gh repo edit "$REPO" --add-topic webcam --add-topic linux --add-topic uvc --add-topic elgato-facecam >/dev/null 2>&1 || true
fi
echo
echo "Listo: https://github.com/$REPO"
echo "GitHub está construyendo los paquetes (pestaña Actions, unos 10 minutos)."
echo "Aparecerán en: https://github.com/$REPO/releases"
echo "Instalar desde la terminal:"
echo "  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | sh"
