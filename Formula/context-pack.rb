class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.7.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-aarch64-apple-darwin.tar.gz"
      sha256 "63c1f3e32cfb5e3d53b4df54525cd54eba604bbe6eccfee333008daf1bd14bb4"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-x86_64-apple-darwin.tar.gz"
      sha256 "1781a4983a94d7e100419cb73d040637a65d8a67c3e8aa9feaa42abf4bad30e0"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "b7d232cedd3c23e3e2d563757e286c0546f37feb3fd007c5ed3dfbd1c26c481b"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
