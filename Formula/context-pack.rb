class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.4.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.3/context-pack-v0.4.3-aarch64-apple-darwin.tar.gz"
      sha256 "dfd5e03d260c127a928d5e31445511b2d69e443bbfe04be04684e01ce78cdb6b"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.3/context-pack-v0.4.3-x86_64-apple-darwin.tar.gz"
      sha256 "d50811bf2aa467227d950d98e7daa81cf9fde2caf00aff27d4f0cbecb7821baf"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.4.3/context-pack-v0.4.3-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "c1c09d62eb26e001af4c339efad1c6ef4ed069941e148eb898d297030a381cbd"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
