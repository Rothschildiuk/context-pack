class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.2.4"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.4/context-pack-v0.2.4-aarch64-apple-darwin.tar.gz"
      sha256 "50038aab7a28f08577331baea4299ce4ac944d6ac0d4e6b6a5976be1279d3875"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.4/context-pack-v0.2.4-x86_64-apple-darwin.tar.gz"
      sha256 "97d82f557e187282e808405060aa70ea9d458cb75225998adf2bcb34ef3c5e27"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.2.4/context-pack-v0.2.4-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "0e7fe394c7fc9ebdccd8d39f15e0a9cae6041162624c6bebe57e57702b070f97"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
