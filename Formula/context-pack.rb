class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.4.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.1/context-pack-v0.4.1-aarch64-apple-darwin.tar.gz"
      sha256 "43ff3ab540c1b42d5904f344690275931af0e0eb84b3037f1d4398c41f135fe6"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.1/context-pack-v0.4.1-x86_64-apple-darwin.tar.gz"
      sha256 "238e90b0b5f5f2a0304f372f808a00d5bc2b94bff3ba79fd5a5575b3d1578b51"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.1/context-pack-v0.4.1-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "41482e9f64de37c1f1dc9c6f5c5a98f99d07317ce7e8a496393e9d0b05148393"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
