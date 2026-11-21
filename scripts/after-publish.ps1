# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
#
# The steps after a release is published on GitHub (all files uploaded; scripts/release.ps1
# -Publish runs this at its end, and it can be run again by itself):
#   1. checks that the latest release in sookera/dobo-releases is this version and carries every
#      file the website and Dobo's updates use (the site's build stops without help.zip);
#   2. asks Cloudflare to rebuild dobo.sookera.com (its deploy hook), so the site shows this
#      version and its guides. The hook's address is secret: read from the gitignored
#      signing/deploy-hook.txt, else from the Sookera vault (secrets/web/deploy-hook-dobo.txt.age,
#      next to this project or at SOOKERA_VAULT, decrypted in memory with the studio key file
#      named by SOOKERA_STUDIO_KEY, see the vault's README). Without either,
#      this step is skipped with a warning (the site then rebuilds on its next push);
#   3. publishes the public source copy of this release to sookera/dobo
#      (scripts/publish-source.ps1: one commit "Dobo <version>", from HEAD, checked first).
#
# Step 3 pushes with git, as this folder's GitHub account (Zak-SKR, from git's settings for the folder).
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/after-publish.ps1
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$cargo = Get-Content -Raw (Join-Path $root "src-tauri/Cargo.toml")
$version = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if (-not $version) { throw "Couldn't read the version from src-tauri/Cargo.toml." }
$base = "https://github.com/sookera/dobo-releases/releases/latest/download"

# 1. The release is out, with its files.
$latest = Invoke-RestMethod "$base/latest.json"
if ($latest.version -ne $version) { throw "The latest release is $($latest.version), not ${version}: publish it first." }
foreach ($f in @("Dobo-Setup.exe", "Dobo.exe", "help.zip")) {
    $r = Invoke-WebRequest -Method Head -UseBasicParsing "$base/$f"
    if ($r.StatusCode -ne 200) { throw "The latest release has no $f." }
}
Write-Host "Release $version is out, with Dobo-Setup.exe, Dobo.exe, latest.json and help.zip."

# 2. Rebuild the website.
$hook = $null
$plain = Join-Path $root "signing/deploy-hook.txt"
if (Test-Path $plain) {
    $hook = (Get-Content -Raw $plain).Trim()
} else {
    $vault = Join-Path (Split-Path -Parent (Split-Path -Parent $root)) "Sookera Studio/sookera-vault/secrets/web/deploy-hook-dobo.txt.age"
    if ($env:SOOKERA_VAULT) { $vault = Join-Path $env:SOOKERA_VAULT "secrets/web/deploy-hook-dobo.txt.age" }
    $studioKey = $env:SOOKERA_STUDIO_KEY
    $age = (Get-Command age -ErrorAction SilentlyContinue).Source
    if (-not $age) { $age = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages\FiloSottile.age_Microsoft.Winget.Source_8wekyb3d8bbwe\age\age.exe' }
    if ($studioKey -and (Test-Path $vault) -and (Test-Path $studioKey) -and (Test-Path $age)) {
        # Decrypted to memory only (never written to disk, never printed).
        $hook = (& $age -d -i $studioKey $vault | Out-String).Trim()
        if ($LASTEXITCODE -ne 0) { $hook = $null }
    }
}
if (-not $hook) {
    Write-Warning "No deploy hook for dobo.sookera.com (signing/deploy-hook.txt or the vault's web/deploy-hook-dobo.txt): the site wasn't asked to rebuild."
} else {
    if ($hook -notmatch '^https://') { throw "The deploy hook isn't an https:// address: check it in the vault." }
    Invoke-RestMethod -Method Post -Uri $hook | Out-Null
    Remove-Variable hook
    Write-Host "dobo.sookera.com is rebuilding (a few minutes)."
}

# 3. The public source copy (sookera/dobo).
& (Join-Path $PSScriptRoot "publish-source.ps1")
