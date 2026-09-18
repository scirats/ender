class Ender < Formula
  desc "Cross-platform Minecraft NeoForge manager"
  homepage "https://github.com/scirats/ender"
  url "https://github.com/scirats/ender/archive/refs/tags/v0.1.0.tar.gz"
  version "0.1.0"
  sha256 "REPLACE_WITH_RELEASE_SOURCE_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--root", prefix, "."
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/ender --version", 0)
  end
end
