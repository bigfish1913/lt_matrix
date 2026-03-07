# typed: false
# frozen_string_literal: true

# Homebrew formula for ltmatrix
# Install with: brew install ltmatrix
# Or from tap: brew install bigfish/ltmatrix/ltmatrix
#
# This formula downloads pre-built binaries from GitHub Releases.
# For building from source, use: cargo install ltmatrix

class Ltmatrix < Formula
  desc "High-performance, cross-platform long-time agent orchestrator"
  homepage "https://github.com/bigfish/ltmatrix"
  license "MIT"
  version "0.1.0"

  # Binary downloads from GitHub Releases
  # The version is automatically updated by the release workflow
  if Hardware::CPU.intel?
    if OS.mac?
      url "https://github.com/bigfish/ltmatrix/releases/download/v#{version}/ltmatrix-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000" # Updated on release
    elsif OS.linux?
      # Use musl build for better compatibility (no glibc dependency)
      url "https://github.com/bigfish/ltmatrix/releases/download/v#{version}/ltmatrix-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000" # Updated on release
    end
  elsif Hardware::CPU.arm?
    if OS.mac?
      url "https://github.com/bigfish/ltmatrix/releases/download/v#{version}/ltmatrix-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000" # Updated on release
    elsif OS.linux?
      url "https://github.com/bigfish/ltmatrix/releases/download/v#{version}/ltmatrix-#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000" # Updated on release
    end
  end

  head do
    url "https://github.com/bigfish/ltmatrix.git", branch: "main"
    depends_on "rust" => :build
  end

  # No runtime dependencies - binary is statically linked
  # For musl builds, all dependencies are embedded

  livecheck do
    url :stable
    strategy :github_latest
  end

  def caveats
    <<~EOS
      ltmatrix requires a code agent backend to be installed:

        Claude Code CLI (recommended): https://claude.ai/code
        OpenCode: https://github.com/opencode-ai/opencode
        KimiCode: https://kimi.moonshot.cn

      To get started, run:
        ltmatrix --help
        ltmatrix "build a REST API"
    EOS
  end

  def install
    if build.head?
      # Build from source
      system "cargo", "install", "--locked", "--root", prefix, "--path", "."
    else
      # Install pre-built binary
      bin.install "ltmatrix"
    end

    # Generate and install shell completions
    generate_completions

    # Install man page if generating from binary
    if (man1/"ltmatrix.1").exist?
      man1.install "ltmatrix.1"
    end
  end

  def generate_completions
    return unless bin/"ltmatrix".exist?

    # Generate completions for supported shells
    %w[bash fish zsh].each do |shell|
      begin
        output = Utils.safe_popen_read(bin/"ltmatrix", "completions", shell)
        case shell
        when "bash"
          (bash_completion/"ltmatrix").write output
        when "fish"
          (fish_completion/"ltmatrix.fish").write output
        when "zsh"
          (zsh_completion/"_ltmatrix").write output
        end
      rescue StandardError
        # Completions not available, skip silently
        nil
      end
    end
  end

  test do
    # Test version output
    assert_match "ltmatrix #{version}", shell_output("#{bin}/ltmatrix --version")

    # Test help output
    assert_match "Long-Time Agent Orchestrator", shell_output("#{bin}/ltmatrix --help")

    # Test that binary runs
    assert_match "Usage:", shell_output("#{bin}/ltmatrix 2>&1", 1)
  end
end
