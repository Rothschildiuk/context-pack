class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.3.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.0/context-pack-v0.3.0-aarch64-apple-darwin.tar.gz"
      sha256 "9f2767ad9be12d78bb4c30f0c1ea06656254725e83e3939c988068ecfff40320"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.0/context-pack-v0.3.0-x86_64-apple-darwin.tar.gz"
      sha256 "4f35e53589a7ba7fd2fcd5ed35ec8b1a4b3440b9e672b5312c934a7d19bc89dd"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.0/context-pack-v0.3.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "d6e592ef63d836bf02870dd216a3d56a8a91bfb6488af58983c7d0391be297ef"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
