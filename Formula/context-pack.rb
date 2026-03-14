class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.4.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.3/context-pack-v0.4.3-aarch64-apple-darwin.tar.gz"
      sha256 "6f928105b7640b9229a28e1b9a8236bb339b06fa12e42e775ceb4e238164f8ec"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.3/context-pack-v0.4.3-x86_64-apple-darwin.tar.gz"
      sha256 "996e2cbd627605b2adbf3f7c4e0a910239fb4c0c223b881dc36b62756ac0f583"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.3/context-pack-v0.4.3-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "f779a7379b2bbf9479eb590a8b976c7a8a341674d479e2abddc149b414ad89e5"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
