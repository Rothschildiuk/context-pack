class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.7.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-aarch64-apple-darwin.tar.gz"
      sha256 "911d11cf28d4c54de8937108ba6ea2d793d7db30853ccdc7b75bfc939762153c"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-x86_64-apple-darwin.tar.gz"
      sha256 "f76ee6b1344036b55a4676d916bf1e431c6237495f06b4133c7338a755dbe33a"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.7.0/context-pack-v0.7.0-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "9f2eede8e6afc508d85b45ec6121a2467a25df312ebc9e1ffa6a702d7b1afec5"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
