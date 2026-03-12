class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.2.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.1/context-pack-v0.2.1-aarch64-apple-darwin.tar.gz"
      sha256 "74a454fab7f7e760fd239c25f055e8e9d9959d71894ffdcfff94e7389f25096f"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.1/context-pack-v0.2.1-x86_64-apple-darwin.tar.gz"
      sha256 "b1ab79b897263180536a3e0a162ff02091f29e3f077259bf0937606d602f342e"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.1/context-pack-v0.2.1-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "7007bab79fd1667135fbbe91d8cfc6164670152dc2271c27c088ea584228296e"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
