class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.2.5"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.5/context-pack-v0.2.5-aarch64-apple-darwin.tar.gz"
      sha256 "ba440454e9044d1a699e37f3f3b0671d148cf558768d0073c8a7129f145317fa"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.5/context-pack-v0.2.5-x86_64-apple-darwin.tar.gz"
      sha256 "a3d7711552d2c6d5e9760ee1630f70086975e0ae1d7e4e2c61e9ad86aef03325"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.5/context-pack-v0.2.5-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "f39b2829eb5be3ebc2bfeb4812b53568e5f6694ef7660528370b1a31a6f79546"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
