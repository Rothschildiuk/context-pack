class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.4.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.2/context-pack-v0.4.2-aarch64-apple-darwin.tar.gz"
      sha256 "449c4ac87e7d0309970e5deebb3e55c6bb2cad33cbcede0587b2f86ad3feaf2a"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.2/context-pack-v0.4.2-x86_64-apple-darwin.tar.gz"
      sha256 "ee14f2c5b77436cbb25895909215c7ccc8e1e7913552cfd4d1d5f293b98cef29"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.2/context-pack-v0.4.2-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "c6434aa7bff66620dd6df2403b01f16bc28d691d024c0fb80879135c924edd91"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
