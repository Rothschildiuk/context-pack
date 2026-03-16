class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.5.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-aarch64-apple-darwin.tar.gz"
      sha256 "9eced6f30b2c53fe9f180c324701bb3bfe624fe546b00f49dd225c848fb46e7e"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-x86_64-apple-darwin.tar.gz"
      sha256 "fd838fe0615866b59a8b503a6348184118f8ccec23cbe8928d6d43c5a3de9635"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "bbda871d3f17ca8ca01af91878937b236e628c97e26596a6f16897d3cfc10d03"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
