class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.6.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.6.0/context-pack-v0.6.0-aarch64-apple-darwin.tar.gz"
      sha256 "cb5d633103c79b72fc38f0ba8c34abaea8e479efbb5ea2ad80011edba73ee23d"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.6.0/context-pack-v0.6.0-x86_64-apple-darwin.tar.gz"
      sha256 "4ebd79a1ecdb8287f97802a5b83bcda98d285a7f614f0f27f5ee988a0d92ab3a"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.6.0/context-pack-v0.6.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "b016202e69093cd4e984acbe2b83576b119c9e8512c5fa39e5aaf8c206899a96"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
