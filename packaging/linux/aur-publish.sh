#!/usr/bin/env bash
# RFC-081: prepare a real, verified PKGBUILD for AUR publication.
#
# This script only ever writes into $WORK_DIR - it never touches the AUR
# and never commits to this repository. It is the one place both trigger
# paths (a GitHub release, or an owner-dispatched recipe fix) and both
# push modes (real, dry-run) share code: the workflow's validation steps
# (makepkg, pacman -U, namcap, .SRCINFO) run identically afterwards in all
# four combinations, and only the final push step differs.
#
# Usage: aur-publish.sh <release|recipe-fix> <work-dir>
#   release:    $RELEASE_TAG must be set - the tag the GitHub release
#               was published against. Requires pkgver == $RELEASE_TAG
#               and pkgrel == 1 (a new upstream version, first packaging).
#   recipe-fix: requires pkgver == the AUR's currently published pkgver
#               (no source change - a packaging-only fix) and pkgrel
#               strictly greater than the AUR's (RFC-081 Q3: automation
#               never writes a version component, only refuses a wrong
#               one).
#
# Reads packaging/linux/PKGBUILD from the current checkout (the release
# path's caller must have checked out the released tag, not main - see
# the workflow). Writes the real PKGBUILD (SKIP replaced with a computed
# hash) to $WORK_DIR/PKGBUILD, and leaves $WORK_DIR/aur-current holding
# what the AUR repository carries right now, for the workflow's later
# no-op check.

set -euo pipefail

MODE="${1:?usage: aur-publish.sh <release|recipe-fix> <work-dir>}"
WORK_DIR="${2:?usage: aur-publish.sh <release|recipe-fix> <work-dir>}"
REPO_PKGBUILD="packaging/linux/PKGBUILD"
AUR_HTTPS_URL="https://aur.archlinux.org/forskscope.git"

mkdir -p "$WORK_DIR"

# ── What does the AUR currently carry? Read-only, no credential needed -
#    AUR git repositories are anonymously readable over HTTPS. ──────────
git clone --depth 1 "$AUR_HTTPS_URL" "$WORK_DIR/aur-current" >/dev/null 2>&1
AUR_PKGVER="$(awk -F' = ' '/^\tpkgver = /{print $2; exit}' "$WORK_DIR/aur-current/.SRCINFO")"
AUR_PKGREL="$(awk -F' = ' '/^\tpkgrel = /{print $2; exit}' "$WORK_DIR/aur-current/.SRCINFO")"
if [ -z "$AUR_PKGVER" ] || [ -z "$AUR_PKGREL" ]; then
    echo "::error::could not read pkgver/pkgrel from the AUR's current .SRCINFO"
    exit 1
fi
echo "AUR currently carries pkgver=$AUR_PKGVER pkgrel=$AUR_PKGREL"

# ── What does the checked-out PKGBUILD claim? ───────────────────────────
REPO_PKGVER="$(awk -F= '/^pkgver=/{print $2; exit}' "$REPO_PKGBUILD")"
REPO_PKGREL="$(awk -F= '/^pkgrel=/{print $2; exit}' "$REPO_PKGBUILD")"
echo "Checked-out PKGBUILD has pkgver=$REPO_PKGVER pkgrel=$REPO_PKGREL"

# ── Verify version rules for the mode. Automation never *writes* a
#    version component - it only refuses to publish a wrong one
#    (RFC-081 Q3: "Automation never writes a version component. pkgver
#    and pkgrel both come from a human commit; automation verifies them
#    and refuses to publish when they are wrong.") ─────────────────────
case "$MODE" in
release)
    : "${RELEASE_TAG:?release mode requires RELEASE_TAG}"
    if [ "$REPO_PKGVER" != "$RELEASE_TAG" ]; then
        echo "::error::PKGBUILD pkgver ($REPO_PKGVER) does not match the released tag ($RELEASE_TAG)"
        exit 1
    fi
    if [ "$REPO_PKGREL" != "1" ]; then
        echo "::error::a new release must publish pkgrel=1 (a first packaging of this version), found pkgrel=$REPO_PKGREL"
        exit 1
    fi
    ;;
recipe-fix)
    if [ "$REPO_PKGVER" != "$AUR_PKGVER" ]; then
        echo "::error::a recipe fix must not change pkgver (checked out: $REPO_PKGVER, AUR: $AUR_PKGVER) - cut a release instead"
        exit 1
    fi
    if ! [ "$REPO_PKGREL" -gt "$AUR_PKGREL" ] 2>/dev/null; then
        echo "::error::a recipe fix must increase pkgrel (checked out: $REPO_PKGREL, AUR: $AUR_PKGREL)"
        exit 1
    fi
    ;;
*)
    echo "::error::unknown mode: $MODE (expected release or recipe-fix)"
    exit 1
    ;;
esac

TARGET_PKGVER="$REPO_PKGVER"

# ── The source hash: computed here, never committed to this repository
#    (RFC-081 §1 - pkgver names an unreleased version almost always, so
#    there is no tarball to hash at commit time). Fetched fresh from the
#    tag's own GitHub archive - the same bytes `makepkg` will download on
#    a user's machine. ───────────────────────────────────────────────────
TARBALL_URL="https://github.com/forskscope/forskscope/archive/refs/tags/${TARGET_PKGVER}.tar.gz"
HASH="$(curl -fsSL "$TARBALL_URL" | sha256sum | awk '{print $1}')"
if [ -z "$HASH" ] || [ "${#HASH}" -ne 64 ]; then
    echo "::error::could not compute a sha256 for $TARBALL_URL"
    exit 1
fi
echo "Computed source hash for $TARGET_PKGVER: $HASH"

# ── Write the real PKGBUILD. `sha256sums=('SKIP')` must never reach this
#    point - RFC-081 names it as the check most likely to be skipped as
#    obvious, so it is checked explicitly rather than trusted to the
#    substitution above having worked. ──────────────────────────────────
sed "s/^sha256sums=('SKIP')\$/sha256sums=('${HASH}')/" "$REPO_PKGBUILD" >"$WORK_DIR/PKGBUILD"

if grep -q "sha256sums=('SKIP')" "$WORK_DIR/PKGBUILD"; then
    echo "::error::sha256sums=('SKIP') would reach the AUR - refusing to publish"
    exit 1
fi
if ! grep -qF "sha256sums=('${HASH}')" "$WORK_DIR/PKGBUILD"; then
    echo "::error::templating did not produce the expected hash line"
    exit 1
fi

echo "Prepared $WORK_DIR/PKGBUILD (mode=$MODE, pkgver=$TARGET_PKGVER, pkgrel=$REPO_PKGREL)"
