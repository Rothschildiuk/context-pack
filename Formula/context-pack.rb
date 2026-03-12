class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.2.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.2/context-pack-v0.2.2-aarch64-apple-darwin.tar.gz"
      sha256 "d0e86e05bf4b5d2a529d10f1284075bfb691d803d8e11f66f08d2b10ef35a1c2"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.2/context-pack-v0.2.2-x86_64-apple-darwin.tar.gz"
      sha256 "77ab4706c2a83a8386a84447e1bdc7e2c2db0d42478aafd012cb74f7ed633e0d"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.2/context-pack-v0.2.2-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "7296f0ae96581b8f79a53bd70bb7c5c41a0df05ca4b25f8bb2c52ad02c0928a3"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
