class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.5.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.3/context-pack-v0.5.3-aarch64-apple-darwin.tar.gz"
      sha256 "bbf2557e2fc67caa49175711894420575b1d3f9d0cee9410c56e3754a5363a25"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.3/context-pack-v0.5.3-x86_64-apple-darwin.tar.gz"
      sha256 "4a1fdb56c1e72df37c5e5b46d57ffb7d1fc4b8e3630aa8c2e54a7f625cbd85b4"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.3/context-pack-v0.5.3-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "dd07bb443d742ea891700a3ab3a51d9ee4afa5aba71156f7fbfc8fa5ecf93151"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
