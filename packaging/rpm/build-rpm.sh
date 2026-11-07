#!/bin/sh
# Builds dist/lente-<version>-1.x86_64.rpm (Fedora, openSUSE, RHEL/Alma/Rocky…).
# Install: sudo dnf install ./lente-<version>-1.x86_64.rpm  (openSUSE: sudo zypper install …)
# Needs rpmbuild (Fedora: sudo dnf install rpm-build; Debian/Ubuntu: sudo apt install rpm).
# Usage (from the project folder): sh packaging/rpm/build-rpm.sh
set -eu
cd "$(dirname "$0")/../.."
command -v rpmbuild >/dev/null 2>&1 || { echo "rpmbuild is missing (package rpm-build or rpm)" >&2; exit 1; }
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ -n "${LENTE_SKIP_BUILD:-}" ] || cargo build --release --locked
mkdir -p dist
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
sh packaging/stage.sh "$WORK/stage" /usr /usr/lib/udev/rules.d
rpmbuild -bb \
    --define "_topdir $WORK/rpm" \
    --define "lente_version $VERSION" \
    --define "lente_stage $WORK/stage" \
    packaging/rpm/lente.spec
find "$WORK/rpm/RPMS" -name '*.rpm' -exec cp {} dist/ \;
echo "Done: dist/lente-$VERSION-1.x86_64.rpm"
