const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const REPO = "flagdash/flagdash-cli";
const BINARY_NAME = "flagdash";
const BIN_DIR = path.join(__dirname, "bin");

const PLATFORM_MAP = {
  "darwin-arm64": "flagdash-darwin-arm64.tar.gz",
  "darwin-x64": "flagdash-darwin-amd64.tar.gz",
  "linux-arm64": "flagdash-linux-arm64.tar.gz",
  "linux-x64": "flagdash-linux-amd64.tar.gz",
};

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;
  const key = `${platform}-${arch}`;

  if (!PLATFORM_MAP[key]) {
    console.error(`Unsupported platform: ${platform}/${arch}`);
    console.error(`Supported platforms: ${Object.keys(PLATFORM_MAP).join(", ")}`);
    process.exit(1);
  }

  return key;
}

function httpsGet(url) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "flagdash-cli-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        httpsGet(res.headers.location).then(resolve, reject);
        return;
      }

      if (res.statusCode !== 200) {
        reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        return;
      }

      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    }).on("error", reject);
  });
}

async function fetchLatestVersion() {
  const url = `https://api.github.com/repos/${REPO}/releases/latest`;
  const data = await httpsGet(url);
  const release = JSON.parse(data.toString());
  return release.tag_name;
}

async function main() {
  const platformKey = getPlatformKey();
  const artifactName = PLATFORM_MAP[platformKey];

  console.log(`Detected platform: ${platformKey}`);

  let version;
  try {
    version = await fetchLatestVersion();
  } catch (err) {
    console.error(`Failed to fetch latest release: ${err.message}`);
    process.exit(1);
  }

  console.log(`Installing ${BINARY_NAME} ${version}...`);

  const downloadUrl = `https://github.com/${REPO}/releases/download/${version}/${artifactName}`;

  let archiveBuffer;
  try {
    console.log(`Downloading ${downloadUrl}...`);
    archiveBuffer = await httpsGet(downloadUrl);
  } catch (err) {
    console.error(`Failed to download binary: ${err.message}`);
    process.exit(1);
  }

  if (!fs.existsSync(BIN_DIR)) {
    fs.mkdirSync(BIN_DIR, { recursive: true });
  }

  const tmpFile = path.join(BIN_DIR, artifactName);
  fs.writeFileSync(tmpFile, archiveBuffer);

  try {
    if (artifactName.endsWith(".tar.gz")) {
      execSync(`tar -xzf "${tmpFile}" -C "${BIN_DIR}"`, { stdio: "pipe" });
    } else if (artifactName.endsWith(".zip")) {
      execSync(`unzip -o "${tmpFile}" -d "${BIN_DIR}"`, { stdio: "pipe" });
    }
  } catch (err) {
    console.error(`Failed to extract archive: ${err.message}`);
    process.exit(1);
  }

  fs.unlinkSync(tmpFile);

  const binaryPath = path.join(BIN_DIR, BINARY_NAME);
  if (!fs.existsSync(binaryPath)) {
    console.error(`Binary not found after extraction: ${binaryPath}`);
    process.exit(1);
  }

  fs.chmodSync(binaryPath, 0o755);

  console.log(`${BINARY_NAME} ${version} installed successfully.`);
}

main();
