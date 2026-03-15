class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.5.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.0/context-pack-v0.5.0-aarch64-apple-darwin.tar.gz"
      sha256 "3f6375cf8b15dc364aa173e794f69539fe8128bbeec1ccba111b30e90df74b53"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.0/context-pack-v0.5.0-x86_64-apple-darwin.tar.gz"
      sha256 "50b40e887e40d0a288896203a53096921fa97076486ecd538a68f696b0cf4ef0"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.0/context-pack-v0.5.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "5c7bc9d212d3457772f488059bc25999d5dbdfa7cffc968b06572efe61ba94b9"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
