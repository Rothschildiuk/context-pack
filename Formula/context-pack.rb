class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.7.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-aarch64-apple-darwin.tar.gz"
      sha256 "6f2013101fe88f12334209cdc25cd07454cb03345eb290b14b21b6078def39c6"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-x86_64-apple-darwin.tar.gz"
      sha256 "91a8d55035b04e8879e97ffb8a39c003974478b5551988bc5a17b1476bd56d63"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "aca9c5dcb6019c7fdf36c84d54b016027a28386270374585dc3c988ecd18bb16"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
