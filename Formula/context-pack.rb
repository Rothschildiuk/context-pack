class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.5.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-aarch64-apple-darwin.tar.gz"
      sha256 "56ba8b570f97831c6b66b15dda5adfab31f898fd4896516a9770c1872cf47c16"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-x86_64-apple-darwin.tar.gz"
      sha256 "208cf01f60d5f0f048deb14aa1ad74203c7ac569405c95d3a6045015e4589f63"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "1f018f9eb3a64c5cb3c322c5c9787ca9f4b59d5de5b00ad859ef2a58b12276f7"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
