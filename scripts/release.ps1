# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Sookera. Dobo by Sookera; see NOTICE.md for the additional terms.
# Makes a Dobo release (modelled on Pako's scripts/release.ps1): builds Dobo.exe and Dobo Setup,
# signs Dobo.exe's update manifest, and puts everything in dist/<version>/:
#
#   Dobo.exe        the portable Dobo, and the file Dobo's own updates download (both kinds)
#   Dobo-Setup.exe  the installer (per user, no admin rights): the same Dobo.exe inside
#   latest.json     what Dobo's update check reads (version, SHA-256, size, notes, signature)
#   help.zip        the guides: docs/help/*.md flat at the zip root (README.md is the index), and
#                   docs/help/fr/*.md in fr/. dobo.sookera.com builds its help pages from the
#                   latest release's help.zip (its root), so the site matches this Dobo.
#   LICENSE         the GNU GPL version 3, and NOTICE.md: the copyright and Sookera's additional
#   NOTICE.md       terms (they go with every copy; the installer puts them next to Dobo.exe too)
#
#   1. Raise the version in src-tauri/Cargo.toml, src-tauri/tauri.conf.json AND package.json (the
#      same number), and give docs/help/whats-new.md a '## <version>' section.
#   2. powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1 [-Notes "What's new"]
#      runs every test (vitest, svelte-check, cargo test, cargo clippy with no warnings) and checks
#      the Rust dependencies (cargo audit: no known vulnerability in Cargo.lock, its warnings only
#      reported; cargo deny 0.19 or newer, on what is built for Windows: advisories, licences that
#      may ship in a GPL-3.0 program, banned crates and where crates come from,
#      src-tauri/deny.toml). Install both once with:
#        cargo install cargo-audit cargo-deny --locked
#      Without them a build only warns, and can't be published; with -Publish a missing tool
#      stops the release, unless -SkipSupplyChecks is given (loudly warned, for an emergency
#      only). Then the npm dependencies (scripts/check-npm-audit.mjs, Node 24.2 or newer:
#      npm audit, online): any known vulnerability in what ships stops the release, and in the
#      development tools only high and critical do (moderate and low are only reported). Offline
#      it stops too, unless -SkipSupplyChecks; an older Node is a missing tool, as above.
#      It builds and checks everything in dist/<version>/, and publishes nothing.
#      dist/<version>/build.json (never uploaded) records the commit it was built from, whether
#      the working tree was clean, whether the dependencies were checked, and each file's SHA-256.
#   3. Then, with -Publish (or the same command again with -Publish -SkipBuild). Refused unless
#      the working tree is clean (everything committed, docs/help too), and dist/<version> was
#      built, tested and its dependencies checked, from that very commit with a clean tree, and is
#      unchanged since:
#        - a DRAFT release tagged v<version> in sookera/dobo-releases, with the files above (only
#          once the repo is checked to have immutable releases on);
#        - every file checked on the draft (names, sizes and GitHub's SHA-256), THEN the release
#          is published: no file can be added or changed once it is out;
#        - scripts/after-publish.ps1: checks the release is the latest, asks the website to
#          rebuild (the deploy hook), then publishes the public source copy to sookera/dobo
#          (scripts/publish-source.ps1).
#      GitHub as Zak-SKR for these commands only (GH_TOKEN from `gh auth token --user Zak-SKR`):
#      never a global account switch.
#
# The signing key is %USERPROFILE%\.dobo-signing\dobo-signing.key (or DOBO_SIGNING_DIR), outside
# the project; its backup is in the Sookera vault (secrets/dobo/dobo-signing.key.age). Without it
# Dobo can't take updates any more.
param(
    [string]$Notes = "",
    [switch]$Publish,
    [switch]$SkipBuild,
    # Publishes even though cargo audit or cargo deny isn't installed (or too old), npm audit
    # can't reach the registry (offline) or Node is older than 24.2: those dependencies aren't
    # checked. For an emergency only; said loudly.
    [switch]$SkipSupplyChecks
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$repo = "sookera/dobo-releases"

# Runs a native command (git, gh) that may write to its error output: Windows PowerShell turns
# those lines into errors that stop this script ($ErrorActionPreference "Stop"), even redirected
# with 2>$null, so `gh release view` of a release that isn't there yet would end it. Its exit code
# is in $LASTEXITCODE as usual.
function Invoke-Native([scriptblock]$Run) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try { & $Run } finally { $ErrorActionPreference = $saved }
}

# A warning that can't be missed in a long log: the dependencies weren't checked.
function Write-Loud([string]$Text) {
    $bar = "!" * 78
    Write-Host ""
    Write-Host $bar -ForegroundColor Black -BackgroundColor Red
    Write-Host "  $Text" -ForegroundColor Black -BackgroundColor Red
    Write-Host $bar -ForegroundColor Black -BackgroundColor Red
    Write-Host ""
}

# A dependency check whose tool isn't there (or is too old): stops a release being published
# (unless -SkipSupplyChecks); a build that won't be published only warns.
function Skip-SupplyCheck([string]$Why) {
    if ($Publish -and -not $SkipSupplyChecks) {
        throw "$Why A release is never published without it: install it, or (in an emergency only) give -SkipSupplyChecks."
    }
    if ($SkipSupplyChecks) {
        Write-Loud "-SkipSupplyChecks: $Why The dependencies were NOT checked."
    } else {
        Write-Warning "$Why This build can be tried, never published."
    }
}

# The commit this tree is at, and whether everything in it is committed (untracked files count;
# the gitignored PLAN.md, design/, build/ and dist/ don't).
function Get-TreeState {
    $head = Invoke-Native { git -C $root rev-parse HEAD 2>$null }
    if ($LASTEXITCODE -ne 0 -or -not $head) { throw "Couldn't read the commit (git rev-parse HEAD)." }
    $changes = @(Invoke-Native { git -C $root status --porcelain --untracked-files=all 2>$null })
    if ($LASTEXITCODE -ne 0) { throw "Couldn't read the working tree (git status)." }
    return [pscustomobject]@{ Commit = "$head".Trim(); Clean = ($changes.Count -eq 0); Changes = $changes }
}
$cargo = Get-Content (Join-Path $root "src-tauri/Cargo.toml") -Raw
$version = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if (-not $version) { throw "Couldn't read the version from src-tauri/Cargo.toml." }
$conf = Get-Content (Join-Path $root "src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json
if ($conf.version -ne $version) {
    throw "Cargo.toml says $version but tauri.conf.json says $($conf.version): make them the same first."
}
$package = Get-Content (Join-Path $root "package.json") -Raw | ConvertFrom-Json
if ($package.version -ne $version) {
    throw "Cargo.toml says $version but package.json says $($package.version): make them the same first."
}
# What's new: the site and the guides show this version's section.
$whatsNew = Get-Content (Join-Path $root "docs/help/whats-new.md") -Raw -Encoding UTF8
if ($whatsNew -match '(?m)^##\s+Next version\s*$') {
    throw "docs/help/whats-new.md still has '## Next version': rename that heading to '## $version' first."
}
$sectionAt = '(?ms)^##[ \t]+(Dobo[ \t]+)?' + [regex]::Escape($version) + '[ \t]*\r?$(.*?)(?=^##[ \t]|\z)'
$section = [regex]::Match($whatsNew, $sectionAt)
if (-not $section.Success) {
    throw "docs/help/whats-new.md has no '## $version' section."
}
# The notes in latest.json (shown in Settings › Updates): given, or the section's first paragraph.
if (-not $Notes) {
    $Notes = (($section.Groups[2].Value -split '\r?\n\s*\r?\n') | ForEach-Object { $_.Trim() } | Where-Object { $_ -and -not $_.StartsWith("#") -and -not $_.StartsWith("-") } | Select-Object -First 1)
    if (-not $Notes) { $Notes = "" }
    $Notes = ($Notes -replace '\*\*', '' -replace '\s+', ' ').Trim()
}
# The signing key must be there before anything is built.
$keyDir = if ($env:DOBO_SIGNING_DIR) { $env:DOBO_SIGNING_DIR } else { Join-Path $env:USERPROFILE ".dobo-signing" }
if (-not (Test-Path (Join-Path $keyDir "dobo-signing.key"))) {
    throw "No signing key in $keyDir (dobo-signing.key). Restore it from the Sookera vault (secrets/dobo/dobo-signing.key.age, see its README)."
}
# A renamed test copy (src-tauri/src/lib.rs APP_NAME) is never a release.
if ($env:DOBO_TEST_NAME -or $env:DOBO_TEST_IDENTIFIER) {
    throw "DOBO_TEST_NAME or DOBO_TEST_IDENTIFIER is set (a renamed test copy): remove them first."
}
$out = Join-Path $root "dist/$version"
$files = @("Dobo.exe", "Dobo-Setup.exe", "latest.json", "help.zip", "LICENSE", "NOTICE.md")
# What this build was made from (never uploaded).
$buildInfo = Join-Path $out "build.json"

$tree = Get-TreeState
if ($Publish -and -not $tree.Clean) {
    throw "The working tree has changes that aren't committed (git status: $($tree.Changes.Count) files, e.g. $($tree.Changes[0].Trim())). Commit them first (docs/help too): a release is published from a commit."
}

if (-not $SkipBuild) {
    if (-not $tree.Clean) {
        Write-Warning "The working tree has changes that aren't committed ($($tree.Changes.Count) files): this build can be tried, never published. Commit everything, then build again."
    }
    Push-Location $root
    try {
        # Every test first: a release is never built from code that fails one.
        Write-Host "Testing..."
        npx vitest run
        if ($LASTEXITCODE -ne 0) { throw "The interface tests failed (npx vitest run): nothing was built." }
        npm run check
        if ($LASTEXITCODE -ne 0) { throw "svelte-check found problems (npm run check): nothing was built." }
        Push-Location (Join-Path $root "src-tauri")
        try {
            cargo test --quiet
            if ($LASTEXITCODE -ne 0) { throw "The Rust tests failed (cargo test): nothing was built." }
            cargo clippy --quiet --all-targets -- -D warnings
            if ($LASTEXITCODE -ne 0) { throw "cargo clippy has warnings (cargo clippy --all-targets): nothing was built." }
            # The dependencies, before anything is built: no known vulnerability (the RustSec
            # advisories, fetched fresh), only licences that may ship in a GPL-3.0 program, no
            # banned crate, and crates only from crates.io (deny.toml).
            # cargo audit reads all of Cargo.lock: --target-os / --target-arch only leave out the
            # advisories that name another platform, not the crates only built for one. So its
            # report has warnings for the Linux-only crates (gtk, webkit2gtk... unmaintained):
            # expected. Its warnings (unmaintained, unsound, yanked) never stop the release, only a
            # vulnerability does (cargo audit's own exit code; no --deny, no audit.toml). cargo
            # deny is the check true to the platform: [graph] targets in deny.toml, only what is
            # built for Windows.
            $script:supplyChecked = $true
            if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
                Write-Host "Checking the dependencies for known vulnerabilities (cargo audit)..."
                Invoke-Native { cargo audit --target-os windows --target-arch x86_64 }
                if ($LASTEXITCODE -ne 0) { throw "cargo audit found a known vulnerability in a dependency (see above; its warnings, the Linux-only crates' among them, never stop a release), or couldn't read the advisories (offline?): update the crate (cargo update -p <crate>) or replace it. Nothing was built." }
            } else {
                $script:supplyChecked = $false
                Skip-SupplyCheck "cargo audit isn't installed: the dependencies weren't checked for known vulnerabilities. Install it once with: cargo install cargo-audit --locked."
            }
            # (deny.toml's advisories keys need cargo-deny 0.19 or newer.)
            $denyVersion = $null
            if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
                $denyText = (Invoke-Native { cargo deny --version 2>$null }) | Out-String
                $m = [regex]::Match($denyText, '(\d+)\.(\d+)\.(\d+)')
                if ($m.Success) { $denyVersion = [version]"$($m.Groups[1].Value).$($m.Groups[2].Value).$($m.Groups[3].Value)" }
            }
            if ($denyVersion -and $denyVersion -ge [version]"0.19.0") {
                Write-Host "Checking the dependencies' advisories, licences, bans and sources (cargo deny $denyVersion)..."
                Invoke-Native { cargo deny check }
                if ($LASTEXITCODE -ne 0) { throw "cargo deny found a dependency whose licence can't ship in a GPL-3.0 program, a known vulnerability, a banned crate or one from an unknown source (see above; src-tauri/deny.toml). Nothing was built." }
            } else {
                $script:supplyChecked = $false
                $why = if ($denyVersion) { "cargo deny $denyVersion is too old (deny.toml needs 0.19 or newer): the dependencies' licences, bans and sources weren't checked. Update it with: cargo install cargo-deny --locked." } else { "cargo deny isn't installed: the dependencies' licences, bans and sources weren't checked. Install it once with: cargo install cargo-deny --locked." }
                Skip-SupplyCheck $why
            }
        } finally { Pop-Location }
        # The npm dependencies (npm audit, GitHub's advisories fetched fresh): any known
        # vulnerability in what ships stops the release; in the development tools only, high and
        # critical do, moderate and low are only reported. Offline, it stops too, unless
        # -SkipSupplyChecks (then it's said loudly, and this build is marked unchecked).
        # (Without node the exit code would be the last command's: never read as a pass.)
        if (-not (Get-Command node -ErrorAction SilentlyContinue)) { throw "node isn't on the PATH: the npm dependencies can't be checked (npm audit). Nothing was built." }
        Invoke-Native { node (Join-Path $PSScriptRoot "check-npm-audit.mjs") }
        switch ($LASTEXITCODE) {
            0 { }
            1 { throw "npm audit found a known vulnerability in an npm dependency (see above: any in what ships with Dobo, high or critical in the development tools): update the package (npm update <package>, or a newer version in package.json) or replace it. Nothing was built." }
            3 {
                if (-not $SkipSupplyChecks) { throw "npm audit couldn't reach the npm registry (offline?): a release checks its npm dependencies online. Connect, then try again (or, in an emergency only, give -SkipSupplyChecks). Nothing was built." }
                $script:supplyChecked = $false
                Skip-SupplyCheck "npm audit couldn't reach the npm registry (offline?): the npm dependencies weren't checked for known vulnerabilities."
            }
            4 {
                $script:supplyChecked = $false
                Skip-SupplyCheck "Node is older than 24.2 (scripts/check-npm-audit.mjs needs it): the npm dependencies weren't checked for known vulnerabilities. Update Node."
            }
            default { throw "npm audit gave no report that could be judged (see above; scripts/check-npm-audit.mjs exit code $LASTEXITCODE). Nothing was built." }
        }
        Write-Host "Building Dobo $version..."
        # Builds Dobo.exe and the installer (bundle.targets in tauri.conf.json: NSIS only). The
        # installer holds the very same Dobo.exe, so one update file serves both.
        npx tauri build
        if ($LASTEXITCODE -ne 0) { throw "The build failed." }
        $exe = Join-Path $root "src-tauri/target/release/Dobo.exe"
        $setup = Join-Path $root "src-tauri/target/release/bundle/nsis/Dobo_${version}_x64-setup.exe"
        if (-not (Test-Path $exe)) { throw "The build made no Dobo.exe ($exe)." }
        if (-not (Test-Path $setup)) { throw "The build made no installer ($setup)." }
        $bytes = [System.IO.File]::ReadAllBytes($exe)
        # Dobo.exe takes Windows' DLLs from System32 only (src-tauri/build.rs, system_dlls_only): a
        # file named like one of them next to a portable Dobo.exe must never be loaded. Read from
        # the exe itself (its load configuration, DependentLoadFlags = 0x800).
        $pe = [BitConverter]::ToInt32($bytes, 0x3C)
        $opt = $pe + 24
        $cfgRva = [BitConverter]::ToUInt32($bytes, $opt + 112 + 80)
        $sec = $opt + [BitConverter]::ToUInt16($bytes, $pe + 20)
        $cfgAt = -1
        for ($i = 0; $i -lt [BitConverter]::ToUInt16($bytes, $pe + 6); $i++) {
            $va = [BitConverter]::ToUInt32($bytes, $sec + 12)
            if ($cfgRva -ge $va -and $cfgRva -lt $va + [BitConverter]::ToUInt32($bytes, $sec + 8)) {
                $cfgAt = $cfgRva - $va + [BitConverter]::ToUInt32($bytes, $sec + 20)
            }
            $sec += 40
        }
        if ([BitConverter]::ToUInt16($bytes, $opt) -ne 0x20B -or $cfgAt -lt 0 -or [BitConverter]::ToUInt16($bytes, $cfgAt + 0x4E) -ne 0x800) {
            throw "Dobo.exe would load DLLs from its own folder (DependentLoadFlags isn't 0x800): check system_dlls_only in src-tauri/build.rs, then build again."
        }
        # No personal paths in the program: Rust keeps source paths for its error messages, and
        # only the local src-tauri/.cargo/config.toml (--remap-path-prefix) keeps this PC's out.
        $personal = @($root, $root.Replace([char]92, [char]47), $env:USERPROFILE, $env:USERPROFILE.Replace([char]92, [char]47))
        if ($env:USERNAME.Length -ge 4) { $personal += $env:USERNAME }
        $texts = @(
            [System.Text.Encoding]::GetEncoding(28591).GetString($bytes),
            [System.Text.Encoding]::Unicode.GetString($bytes),
            [System.Text.Encoding]::Unicode.GetString($bytes, 1, $bytes.Length - 1)
        )
        Remove-Variable bytes
        foreach ($p in $personal | Where-Object { $_ }) {
            foreach ($t in $texts) {
                if ($t.IndexOf($p, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
                    throw "Dobo.exe contains '$p': it must not ship with this PC's paths. Check src-tauri/.cargo/config.toml (--remap-path-prefix for your user folder and the project folder) and that RUSTFLAGS isn't set, then build again."
                }
            }
        }
        Remove-Variable texts
        if (Test-Path $out) { Remove-Item $out -Recurse -Force }
        New-Item -ItemType Directory -Force $out | Out-Null
        Copy-Item $exe (Join-Path $out "Dobo.exe") -Force
        Copy-Item $setup (Join-Path $out "Dobo-Setup.exe") -Force
        # The guides for the website: flat at the root (what the site reads), French in fr/.
        $helpDir = Join-Path $root "docs/help"
        $help = @(Get-ChildItem $helpDir -Filter *.md -File | Where-Object { $_.Name -ne "AUDIT.md" })
        if (-not ($help | Where-Object { $_.Name -eq "README.md" })) { throw "docs/help/README.md (the guide index) is missing." }
        if (-not ($help | Where-Object { $_.Name -eq "whats-new.md" })) { throw "docs/help/whats-new.md is missing." }
        $fr = @(Get-ChildItem (Join-Path $helpDir "fr") -Filter *.md -File -ErrorAction SilentlyContinue)
        $zip = Join-Path $out "help.zip"
        Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
        $archive = [System.IO.Compression.ZipFile]::Open($zip, [System.IO.Compression.ZipArchiveMode]::Create)
        try {
            # (Entry names with "/", as every unzip reads them.)
            foreach ($f in $help) { [void][System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $f.FullName, $f.Name, [System.IO.Compression.CompressionLevel]::Optimal) }
            foreach ($f in $fr) { [void][System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $f.FullName, "fr/" + $f.Name, [System.IO.Compression.CompressionLevel]::Optimal) }
        } finally { $archive.Dispose() }
        # The licence and its additional terms go with every copy (the portable Dobo is one file).
        foreach ($legal in @("LICENSE", "NOTICE.md")) {
            $from = Join-Path $root $legal
            if (-not (Test-Path $from)) { throw "$legal is missing: a release must carry the licence." }
            Copy-Item $from (Join-Path $out $legal) -Force
        }
        Write-Host "Signing..."
        Push-Location (Join-Path $root "src-tauri")
        try {
            # (A development build of the same source: its console shows what it did.)
            cargo run --quiet -- --release sign (Join-Path $out "Dobo.exe") $version $Notes
            if ($LASTEXITCODE -ne 0) { throw "Signing failed." }
        } finally { Pop-Location }
        # What it was built from: clean only if nothing changed before or during the build.
        $after = Get-TreeState
        $hashes = [ordered]@{}
        foreach ($f in $files) { $hashes[$f] = (Get-FileHash (Join-Path $out $f) -Algorithm SHA256).Hash.ToLower() }
        $info = [ordered]@{ version = $version; commit = $tree.Commit; clean = ($tree.Clean -and $after.Clean -and $after.Commit -eq $tree.Commit); tested = $true; supplyChecked = $script:supplyChecked; files = $hashes }
        [System.IO.File]::WriteAllText($buildInfo, ($info | ConvertTo-Json -Depth 4) + "`n", (New-Object System.Text.UTF8Encoding $false))
    } finally { Pop-Location }
}

# Everything is there, and latest.json matches Dobo.exe.
foreach ($f in $files) {
    if (-not (Test-Path (Join-Path $out $f))) { throw "dist/$version/$f is missing: build first (without -SkipBuild)." }
}
$manifest = Get-Content (Join-Path $out "latest.json") -Raw | ConvertFrom-Json
$exeHash = (Get-FileHash (Join-Path $out "Dobo.exe") -Algorithm SHA256).Hash.ToLower()
if ($manifest.version -ne $version -or $manifest.sha256 -ne $exeHash -or [int64]$manifest.size -ne (Get-Item (Join-Path $out "Dobo.exe")).Length) {
    throw "dist/$version/latest.json doesn't match Dobo.exe (version, SHA-256 or size): build again."
}
Write-Host ""
Write-Host "Ready in $out :"
Write-Host ("  " + ($files -join ", "))

$built = $null
if (Test-Path $buildInfo) { $built = Get-Content $buildInfo -Raw | ConvertFrom-Json }
if (-not $Publish) {
    if ($built -and $built.clean -and -not $built.supplyChecked) {
        Write-Host "Nothing published. Built and tested from commit $($built.commit) with a clean tree, but its dependencies weren't checked (cargo audit or cargo deny missing, npm audit offline, or Node older than 24.2; see above): it can't be published. Fix that, then build again."
    } elseif ($built -and $built.clean) {
        Write-Host "Nothing published. Built, tested, from commit $($built.commit) with a clean tree: to publish, the same command with -Publish -SkipBuild."
    } else {
        Write-Host "Nothing published. This build can't be published (built from a tree with changes that weren't committed): commit everything, then build again."
    }
    return
}

# ---------- publishing (only with -Publish) ----------
# Only what was built, tested, from this very commit with nothing uncommitted, and unchanged since.
if (-not $built) { throw "dist/$version has no build.json: build it with this script first (without -SkipBuild)." }
if ($built.version -ne $version -or -not $built.tested) { throw "dist/$version wasn't built and tested by this script for ${version}: build again." }
if (-not $built.clean) { throw "dist/$version was built from a tree with changes that weren't committed: commit everything, then build again." }
if ($built.commit -ne $tree.Commit) { throw "dist/$version was built from commit $($built.commit), but this tree is at $($tree.Commit): build again." }
if (-not $built.supplyChecked) {
    if (-not $SkipSupplyChecks) { throw "dist/$version was built without checking its dependencies (cargo audit or cargo deny wasn't installed, npm audit was offline, or Node was older than 24.2): fix that, then build again (or, in an emergency only, give -SkipSupplyChecks)." }
    Write-Loud "-SkipSupplyChecks: publishing a build whose dependencies were NOT checked (cargo audit / cargo deny / npm audit)."
}
$local = @{}
foreach ($f in $files) {
    $local[$f] = (Get-FileHash (Join-Path $out $f) -Algorithm SHA256).Hash.ToLower()
    if ($built.files.$f -ne $local[$f]) { throw "dist/$version/$f changed since it was built: build again." }
}

# GitHub as Zak-SKR, for these commands only.
$token = Invoke-Native { gh auth token --user Zak-SKR 2>$null }
if ($LASTEXITCODE -ne 0 -or -not $token) { throw "gh has no login for Zak-SKR: gh auth login (it adds that account, nothing else changes)." }
$oldToken = $env:GH_TOKEN
$env:GH_TOKEN = "$token".Trim()
try {
    $tag = "v$version"
    # Immutable releases must be on before anything is published (no file changes afterwards).
    $immutable = Invoke-Native { gh api "repos/$repo/immutable-releases" --jq ".enabled" 2>$null }
    if ($LASTEXITCODE -ne 0 -or "$immutable".Trim() -ne "true") {
        throw "$repo doesn't have immutable releases on (or can't be read as Zak-SKR): turn them on in its Settings > General > Releases first. Nothing was published."
    }
    Invoke-Native { gh release view $tag -R $repo --json tagName 2>$null } | Out-Null
    if ($LASTEXITCODE -eq 0) { throw "$repo already has a release $tag (perhaps a draft left by an earlier try: check it on GitHub). Nothing was published." }
    $paths = $files | ForEach-Object { Join-Path $out $_ }
    $releaseNotes = if ($Notes) { $Notes } else { "Dobo $version" }
    Invoke-Native { gh release create $tag @paths -R $repo --draft --title "Dobo $version" --notes $releaseNotes }
    if ($LASTEXITCODE -ne 0) { throw "Couldn't create the draft release $tag in $repo." }
    # Every file on the draft, with its size and GitHub's own SHA-256, before it is published.
    $draft = Invoke-Native { gh release view $tag -R $repo --json isDraft,assets } | Out-String | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or -not $draft) { throw "Couldn't read the draft $tag back: check it on GitHub (nothing is published)." }
    if (-not $draft.isDraft) { throw "$tag on GitHub isn't a draft: check it there." }
    foreach ($f in $files) {
        $a = @($draft.assets | Where-Object { $_.name -eq $f })
        if ($a.Count -ne 1) { throw "The draft $tag has no $f (or more than one): check it on GitHub (nothing is published)." }
        if ([int64]$a[0].size -ne (Get-Item (Join-Path $out $f)).Length) { throw "The draft's $f doesn't have the right size: check it on GitHub (nothing is published)." }
        if ($a[0].digest -and $a[0].digest -ne "sha256:$($local[$f])") { throw "The draft's $f doesn't have the right SHA-256: check it on GitHub (nothing is published)." }
    }
    if ($draft.assets.Count -ne $files.Count) { throw "The draft $tag has files that weren't uploaded by this script: check it on GitHub (nothing is published)." }
    Invoke-Native { gh release edit $tag -R $repo --draft=false --latest }
    if ($LASTEXITCODE -ne 0) { throw "Couldn't publish the draft $tag (it's still a draft on GitHub)." }
    Write-Host "Published $tag in $repo."
} finally {
    $env:GH_TOKEN = $oldToken
}
& (Join-Path $PSScriptRoot "after-publish.ps1")
