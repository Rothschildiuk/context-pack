class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.5.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.3/context-pack-v0.5.3-aarch64-apple-darwin.tar.gz"
      sha256 "2a517cc0cfa900284a8ac33e2ad6ceec6767ea0fc691f7415216bde27883337c"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.3/context-pack-v0.5.3-x86_64-apple-darwin.tar.gz"
      sha256 "5591caf7c2ce7fc55306da46f0376f98d3da3edf0b92cc060334cd2de10bb6bc"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.3/context-pack-v0.5.3-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "d924dde49c602759d52af4608bf47a5ab5a9a20b57e3cd66e8f4e5a2911a4eb5"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
