#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Validates if a version string follows semantic versioning format
 * @param {string} version - Version string to validate
 * @returns {boolean} - Whether the version is valid
 */
function isValidVersion(version) {
  const semverRegex = /^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?(\+[a-zA-Z0-9.-]+)?$/;
  return semverRegex.test(version);
}

/**
 * Updates version in package.json
 * @param {string} newVersion - New version to set
 */
function updatePackageJson(newVersion) {
  const packageJsonPath = join(__dirname, '..', 'package.json');
  const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));

  const oldVersion = packageJson.version;
  packageJson.version = newVersion;

  writeFileSync(
    packageJsonPath,
    JSON.stringify(packageJson, null, 2) + '\n',
    'utf-8'
  );
  console.log(`✓ Updated package.json: ${oldVersion} → ${newVersion}`);
}

/**
 * Updates version in Cargo.toml
 * @param {string} newVersion - New version to set
 */
function updateCargoToml(newVersion) {
  const cargoTomlPath = join(__dirname, '..', 'src-tauri', 'Cargo.toml');
  let cargoToml = readFileSync(cargoTomlPath, 'utf-8');

  const versionRegex = /^version = "([^"]+)"$/m;
  const match = cargoToml.match(versionRegex);

  if (!match) {
    throw new Error('Could not find version in Cargo.toml');
  }

  const oldVersion = match[1];
  cargoToml = cargoToml.replace(versionRegex, `version = "${newVersion}"`);

  writeFileSync(cargoTomlPath, cargoToml, 'utf-8');
  console.log(`✓ Updated Cargo.toml: ${oldVersion} → ${newVersion}`);
}

/**
 * Updates version in tauri.conf.json
 * @param {string} newVersion - New version to set
 */
function updateTauriConfig(newVersion) {
  const tauriConfigPath = join(__dirname, '..', 'src-tauri', 'tauri.conf.json');
  const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf-8'));

  const oldVersion = tauriConfig.version;
  tauriConfig.version = newVersion;

  writeFileSync(
    tauriConfigPath,
    JSON.stringify(tauriConfig, null, 2) + '\n',
    'utf-8'
  );
  console.log(`✓ Updated tauri.conf.json: ${oldVersion} → ${newVersion}`);
}

/**
 * Main function to update all version files
 */
function main() {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.error('Error: No version specified');
    console.log('Usage: node update-version.js <version>');
    console.log('Example: node update-version.js 0.1.2');
    process.exit(1);
  }

  const newVersion = args[0];

  // Validate version format
  if (!isValidVersion(newVersion)) {
    console.error(`Error: Invalid version format: ${newVersion}`);
    console.log(
      'Version must follow semantic versioning (e.g., 0.1.2, 1.0.0-beta.1)'
    );
    process.exit(1);
  }

  console.log(`\nUpdating version to: ${newVersion}\n`);

  try {
    updatePackageJson(newVersion);
    updateCargoToml(newVersion);
    updateTauriConfig(newVersion);

    console.log('\n✅ All versions updated successfully!\n');
  } catch (error) {
    console.error(`\n❌ Error updating versions: ${error.message}\n`);
    process.exit(1);
  }
}

main();
