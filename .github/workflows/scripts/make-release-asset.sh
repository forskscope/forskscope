#!/usr/bin/env bash
# Builds the release and creates an archive and optionally deploys to GitHub.
set -ex

if [[ -z "$GITHUB_REF" ]]
then
  echo "GITHUB_REF must be set"
  exit 1
fi
# Strip (repo)-refs/tags/ from the start of the ref.
TAG=${GITHUB_REF#*/tags/}

host=$(rustc -Vv | grep ^host: | sed -e "s/host: //g")
target=$2
if [ "$host" != "$target" ]
then
  export "CARGO_TARGET_$(echo $target | tr a-z- A-Z_)_LINKER"=rust-lld
fi
export CARGO_PROFILE_RELEASE_LTO=true

BIN_NAME=patch-hygge
# todo: to mitigate webview failure NO_STRIP is required
env NO_STRIP=1 npm run tauri build --locked -- --target $target

cd src-tauri/target/$target/release

mkdir $BIN_NAME-main
os_tag=$3
case $1 in
  ubuntu*)
    cp $BIN_NAME $BIN_NAME-main/
    asset="$BIN_NAME-$os_tag-$TAG.tar.gz"
    tar czf ../../$asset $BIN_NAME-main
    ;;
  macos*)
    cp $BIN_NAME $BIN_NAME-main/
    asset="$BIN_NAME-$os_tag-$TAG.tar.gz"
    # There is a bug with BSD tar on macOS where the first 8MB of the file are
    # sometimes all NUL bytes. See https://github.com/actions/cache/issues/403
    # and https://github.com/rust-lang/cargo/issues/8603 for some more
    # information. An alternative solution here is to install GNU tar, but
    # flushing the disk cache seems to work, too.
    sudo /usr/sbin/purge
    tar czf ../../$asset $BIN_NAME-main
    ;;
  windows*)
    cp $BIN_NAME.exe $BIN_NAME-main/
    asset="$BIN_NAME-$os_tag-$TAG.zip"
    7z a -w ../../$asset $BIN_NAME-main
    ;;
  *)
    echo "OS should be first parameter, was: $1"
    ;;
esac

cd ../..

if [[ -z "$GITHUB_ENV" ]]
then
  echo "GITHUB_ENV not set, run: gh release upload $TAG src-tauri/target/$asset"
else
  echo "APP_TAG=$TAG" >> $GITHUB_ENV
  echo "APP_ASSET=src-tauri/target/$asset" >> $GITHUB_ENV
fi