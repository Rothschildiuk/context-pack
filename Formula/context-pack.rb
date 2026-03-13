class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.3.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.1/context-pack-v0.3.1-aarch64-apple-darwin.tar.gz"
      sha256 "029fc8f6602b662057b6c7f08ec04ef2677c2042957b6c8c038cf78e5ed9a852"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.1/context-pack-v0.3.1-x86_64-apple-darwin.tar.gz"
      sha256 "8404fa593f74cdb291251af0f9ab327a20d892fbab6e9dd710bf9aca8595d491"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.3.1/context-pack-v0.3.1-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "dcd67be28699e75b5556230e68621cce79366dfad6968801cf0c832e49121922"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
