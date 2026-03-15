class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.4.4"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.4/context-pack-v0.4.4-aarch64-apple-darwin.tar.gz"
      sha256 "5055f6ee89083d3e4543e0024a0193e17904fe65e2ab96bde85fbd1df8687573"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.4/context-pack-v0.4.4-x86_64-apple-darwin.tar.gz"
      sha256 "ca53fc0d3776d4d8496d257d6200639328b91a0e2fcc74f9eebbace40819fb50"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.4/context-pack-v0.4.4-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "08a6595d85c00932c0d05b58e227ee07ff78a15da74f241f42432ceac9047fe9"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
