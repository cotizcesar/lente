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
TAG="v$VERSION"
# Point the version tag at the newest commit. Moving a tag GitHub already published a release
# for would change a released version, so that case asks for a new version number instead.
if [ "$(git rev-parse "$TAG^{commit}" 2>/dev/null)" != "$(git rev-parse HEAD)" ]; then
    if curl -fsS -o /dev/null "https://api.github.com/repos/$REPO/releases/tags/$TAG" 2>/dev/null; then
        echo "Release $TAG is already published: bump the version in Cargo.toml for these changes."
        exit 1
    fi
    git tag -f -a "$TAG" -m "Lente $VERSION" >/dev/null
fi
git push -u origin main
# --force only moves the tag of an unpublished version (checked above); the release workflow
# runs again on the new commit.
git push --force origin "$TAG"
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    gh repo edit "$REPO" --description "Webcam control (Elgato Facecam and UVC) for Linux" \
        --add-topic webcam --add-topic linux --add-topic uvc --add-topic elgato-facecam >/dev/null 2>&1 || true
    echo "Following the release build (Ctrl+C stops watching, not the build)…"
    sleep 5
    RUN=$(gh run list --repo "$REPO" --workflow release.yml --limit 1 --json databaseId -q '.[0].databaseId' 2>/dev/null || true)
    [ -n "$RUN" ] && gh run watch "$RUN" --repo "$REPO" --exit-status || true
fi
echo
echo "Done: https://github.com/$REPO"
echo "GitHub is building the packages (Actions tab, about 10 minutes)."
echo "They will appear at: https://github.com/$REPO/releases"
echo "Install from the terminal:"
echo "  curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | sh"
