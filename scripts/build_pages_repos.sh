#!/usr/bin/env bash
set -euo pipefail

site_dir=$1
deb_dir=$2
rpm_dir=$3
flatpak_repo_dir=$4

apt_root="$site_dir/apt"
yum_root="$site_dir/yum"

mkdir -p "$apt_root/pool/main" "$yum_root/packages"

find "$deb_dir" -type f -name '*.deb' -exec cp {} "$apt_root/pool/main/" \;
find "$rpm_dir" -type f -name '*.rpm' -exec cp {} "$yum_root/packages/" \;

for arch in amd64 arm64; do
  dist_dir="$apt_root/dists/stable/main/binary-$arch"
  mkdir -p "$dist_dir"
  dpkg-scanpackages --arch "$arch" "$apt_root/pool/main" > "$dist_dir/Packages"
  gzip -kf "$dist_dir/Packages"
done

apt-ftparchive release "$apt_root/dists/stable" > "$apt_root/dists/stable/Release"
createrepo_c --update "$yum_root/packages"

if [ -d "$flatpak_repo_dir" ]; then
  mkdir -p "$site_dir/flatpak"
  cp -R "$flatpak_repo_dir"/. "$site_dir/flatpak/"
fi
