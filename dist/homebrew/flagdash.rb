class Flagdash < Formula
  desc "Interactive terminal UI for FlagDash feature flag management"
  homepage "https://flagdash.io"
  license "MIT"
  version "RELEASE_VERSION"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/flagdash/flagdash/releases/download/cli-v#{version}/flagdash-darwin-arm64.tar.gz"
      sha256 "RELEASE_SHA256_MACOS_ARM64"
    else
      url "https://github.com/flagdash/flagdash/releases/download/cli-v#{version}/flagdash-darwin-amd64.tar.gz"
      sha256 "RELEASE_SHA256_MACOS_AMD64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/flagdash/flagdash/releases/download/cli-v#{version}/flagdash-linux-arm64.tar.gz"
      sha256 "RELEASE_SHA256_LINUX_ARM64"
    else
      url "https://github.com/flagdash/flagdash/releases/download/cli-v#{version}/flagdash-linux-amd64.tar.gz"
      sha256 "RELEASE_SHA256_LINUX_AMD64"
    end
  end

  def install
    bin.install "flagdash"
  end

  test do
    system "#{bin}/flagdash", "--version"
  end
end
