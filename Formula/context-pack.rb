class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.2.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.3/context-pack-v0.2.3-aarch64-apple-darwin.tar.gz"
      sha256 "144d6ceac1a50f6266930d3e86c87fc93acfc851bc4f2cf48220755c3b2d95e2"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.3/context-pack-v0.2.3-x86_64-apple-darwin.tar.gz"
      sha256 "2b4b26d4e1b7eebc4a0f43f68ce9cd74fb161873a944d1ac28f6102ea6a8e68a"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.3/context-pack-v0.2.3-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "f67f6fc8a33fe4ba0ca7292018775d56a3064331b3c3c1199f05d355380f055e"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
