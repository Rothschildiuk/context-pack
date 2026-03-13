class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.3.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.2/context-pack-v0.3.2-aarch64-apple-darwin.tar.gz"
      sha256 "05ba5777f806202442c0286d3c1c8b525a44e5cfa03cad516adc445b9f39acc9"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.2/context-pack-v0.3.2-x86_64-apple-darwin.tar.gz"
      sha256 "769c481ee6d5944653c4740ef5d2f43195764c388742c04573ce88f7a1abe6ed"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.2/context-pack-v0.3.2-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "e592a81439143e78359115beed56e1bc54104a1a1a699b71c75981626e16367c"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
