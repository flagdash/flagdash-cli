#!/bin/sh
set -e

REPO="flagdash/flagdash-cli"
BINARY_NAME="flagdash"
INSTALL_DIR="/usr/local/bin"

main() {
  os=$(detect_os)
  arch=$(detect_arch)

  echo "Detected platform: ${os}/${arch}"

  version=$(fetch_latest_version)
  if [ -z "$version" ]; then
    echo "Error: Could not determine latest release version." >&2
    exit 1
  fi

  echo "Installing ${BINARY_NAME} ${version}..."

  artifact_name=$(get_artifact_name "$os" "$arch")
  download_url="https://github.com/${REPO}/releases/download/${version}/${artifact_name}"

  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT

  echo "Downloading ${download_url}..."
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$download_url" -o "${tmpdir}/${artifact_name}"
  elif command -v wget >/dev/null 2>&1; then
    wget -q "$download_url" -O "${tmpdir}/${artifact_name}"
  else
    echo "Error: curl or wget is required." >&2
    exit 1
  fi

  echo "Extracting..."
  case "$artifact_name" in
    *.tar.gz)
      tar -xzf "${tmpdir}/${artifact_name}" -C "$tmpdir"
      ;;
    *.zip)
      unzip -q "${tmpdir}/${artifact_name}" -d "$tmpdir"
      ;;
  esac

  if [ -w "$INSTALL_DIR" ]; then
    cp "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
  else
    echo "Installing to ${INSTALL_DIR} (requires sudo)..."
    sudo cp "${tmpdir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
  fi

  echo ""
  echo "${BINARY_NAME} ${version} installed to ${INSTALL_DIR}/${BINARY_NAME}"
  echo "Run '${BINARY_NAME} --help' to get started."
}

detect_os() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "darwin" ;;
    *)
      echo "Error: Unsupported operating system: $(uname -s)" >&2
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)  echo "amd64" ;;
    aarch64|arm64)  echo "arm64" ;;
    *)
      echo "Error: Unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

fetch_latest_version() {
  url="https://api.github.com/repos/${REPO}/releases/latest"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
  fi
}

get_artifact_name() {
  os="$1"
  arch="$2"

  case "${os}-${arch}" in
    darwin-arm64)  echo "flagdash-darwin-arm64.tar.gz" ;;
    darwin-amd64)  echo "flagdash-darwin-amd64.tar.gz" ;;
    linux-arm64)   echo "flagdash-linux-arm64.tar.gz" ;;
    linux-amd64)   echo "flagdash-linux-amd64.tar.gz" ;;
    *)
      echo "Error: No binary available for ${os}/${arch}" >&2
      exit 1
      ;;
  esac
}

main
