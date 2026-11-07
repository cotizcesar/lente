#!/bin/sh
# Pushes Lente to GitHub (https://github.com/cotizcesar/lente, public) and publishes the current
# version. GitHub then builds every package (tarball, .deb, .rpm, AppImage, Flatpak) and attaches
# them to the release, together with install.sh for terminal installs.
#
# Needs git with your GitHub SSH key. With gh (GitHub CLI) it creates the repository itself
# (signing in if needed); otherwise it asks you to create it empty at https://github.com/new.
# Usage: sh publish.sh
set -eu
cd "$(dirname "$0")"
REPO="cotizcesar/lente"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
command -v git >/dev/null || { echo "git is missing."; exit 1; }

[ -d .git ] || git init -q -b main
git remote get-url origin >/dev/null 2>&1 || git remote add origin "git@github.com:$REPO.git"

if ! git ls-remote origin >/dev/null 2>&1; then
    if command -v gh >/dev/null 2>&1; then
        # GitHub CLI creates the repository; without a session it signs in first (code in the browser).
        gh auth status >/dev/null 2>&1 || gh auth login --hostname github.com --git-protocol ssh --web
        gh repo create "$REPO" --public \
            --description "Webcam control (Elgato Facecam and UVC) for Linux"
    else
        echo "The repository https://github.com/$REPO doesn't exist yet."
        echo "Create it at https://github.com/new named \"lente\", Public, without a README, license"
        echo "or .gitignore. Then press Enter."
        read -r _
        git ls-remote origin >/dev/null 2>&1 || { echo "It still doesn't exist (or your SSH key has no access)."; exit 1; }
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
echo "Done: https://github.com/$REPO"
echo "GitHub is building the packages (Actions tab, about 10 minutes)."
echo "They will appear at: https://github.com/$REPO/releases"
echo "Install from the terminal:"
echo "  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | sh"
