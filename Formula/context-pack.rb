class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.5.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.1/context-pack-v0.5.1-aarch64-apple-darwin.tar.gz"
      sha256 "4cae4be5c0859b4e1c7804f0849c16acd8105ee68502ab8b5b9f8ec14c87a404"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.1/context-pack-v0.5.1-x86_64-apple-darwin.tar.gz"
      sha256 "b1dd2bfac3b5600590463039486fa0fe4a11f11a8b74a5f773e2ee0232603fbb"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.1/context-pack-v0.5.1-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "21b721f6f8fa832f0096ca4dde911b5c274bfb45bc5c2f5772a9c43270bee271"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
