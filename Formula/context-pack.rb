class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.5.4"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.4/context-pack-v0.5.4-aarch64-apple-darwin.tar.gz"
      sha256 "05dbc204e74ebfbdd4cb6ea033c7ca8c9aff95cf30525ef1600efdc47ca807a4"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.4/context-pack-v0.5.4-x86_64-apple-darwin.tar.gz"
      sha256 "7bff7a8e3d3631e440476a27c07ce3cac8e3008e6c3ccbb29812a40cf10bb0cb"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.4/context-pack-v0.5.4-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "95981c2ca711e0472b410fde5ac051b374501bc834a285f59a1e3e2b42611a6d"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
