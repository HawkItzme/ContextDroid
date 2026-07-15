# typed: false
# frozen_string_literal: true

class Contextdroid < Formula
  desc "Conservative Android diagnostics with durable raw-output recovery"
  homepage "https://github.com/HawkItzme/ContextDroid"
  version "0.1.0-alpha.1"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/HawkItzme/ContextDroid/releases/download/v#{version}/contextdroid-aarch64-apple-darwin.tar.gz"
      sha256 "RELEASE_CHECKLIST_REQUIRES_REAL_SHA256"
    else
      url "https://github.com/HawkItzme/ContextDroid/releases/download/v#{version}/contextdroid-x86_64-apple-darwin.tar.gz"
      sha256 "RELEASE_CHECKLIST_REQUIRES_REAL_SHA256"
    end
  end

  def install
    bin.install "contextdroid"
  end

  test do
    assert_match "contextdroid", shell_output("#{bin}/contextdroid --version")
  end
end
