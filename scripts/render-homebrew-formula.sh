#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 4 ]; then
    echo "usage: $0 <repo-slug> <version> <artifacts-dir> <output-path>" >&2
    exit 1
fi

repo_slug=$1
version=$2
artifacts_dir=$3
output_path=$4

base_name="context-pack-v${version}"
darwin_arm64_asset="${base_name}-aarch64-apple-darwin.tar.gz"
darwin_amd64_asset="${base_name}-x86_64-apple-darwin.tar.gz"
linux_amd64_asset="${base_name}-x86_64-unknown-linux-gnu.tar.gz"

read_sha256() {
    local asset_name=$1
    local checksum_file="${artifacts_dir}/${asset_name}.sha256"

    if [ ! -f "$checksum_file" ]; then
        echo "missing checksum file: $checksum_file" >&2
        exit 1
    fi

    awk '{print $1}' "$checksum_file"
}

darwin_arm64_sha=$(read_sha256 "$darwin_arm64_asset")
darwin_amd64_sha=$(read_sha256 "$darwin_amd64_asset")
linux_amd64_sha=$(read_sha256 "$linux_amd64_asset")

cat > "$output_path" <<EOF
class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/${repo_slug}"
  version "${version}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/${repo_slug}/releases/download/v${version}/${darwin_arm64_asset}"
      sha256 "${darwin_arm64_sha}"
    else
      url "https://github.com/${repo_slug}/releases/download/v${version}/${darwin_amd64_asset}"
      sha256 "${darwin_amd64_sha}"
    end
  end

  on_linux do
    url "https://github.com/${repo_slug}/releases/download/v${version}/${linux_amd64_asset}"
    sha256 "${linux_amd64_sha}"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
EOF
