class ContextPack < Formula
  desc "Compact repository context bundles for coding agents"
  homepage "https://github.com/Rothschildiuk/context-pack"
  version "0.5.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-aarch64-apple-darwin.tar.gz"
      sha256 "03a5540b91c2ba862816ea45571ca4dc4d5d007a86c7976dcc81f2bece4f3be7"
    else
      url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-x86_64-apple-darwin.tar.gz"
      sha256 "8f7cf26b96566cff84dcde5c43715736b65471990711c7641c174faa23b0d55e"
    end
  end

  on_linux do
    url "https://github.com/Rothschildiuk/context-pack/releases/download/v0.5.2/context-pack-v0.5.2-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "14f136118fd023a31ad8b4c59da47fc5324bea50abff39764395e621641ae78c"
  end

  def install
    bin.install "context-pack"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/context-pack --version")
  end
end
