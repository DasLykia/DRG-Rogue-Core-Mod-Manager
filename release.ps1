# Publishes a new version on GitHub with Velopack; installed apps then offer it as an update.
#   .\release.ps1 0.2.0
# Needs: Rust (GNU toolchain), vpk (dotnet tool install -g vpk), GitHub CLI logged in (gh auth login).
param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$repo = 'https://github.com/DasLykia/DPG-Rogue-Core-Mod-Manager'
$token = gh auth token

# First run: create the public repo "DPG-Rogue-Core-Mod-Manager" and upload the code.
git remote get-url origin 2>$null | Out-Null
if ($LASTEXITCODE) {
    gh repo create DasLykia/DPG-Rogue-Core-Mod-Manager --public --description 'DPG: Rogue Core Mod Manager - mod manager for Deep Rock Galactic: Rogue Core (Nexus Mods), with Velopack auto-updates' --source . --remote origin --push
    if ($LASTEXITCODE) { throw 'creating the GitHub repo failed' }
}

# The version shown in the app comes from Cargo.toml.
(Get-Content Cargo.toml -Raw) -replace '(?m)^version = ".*"', "version = `"$Version`"" | Set-Content Cargo.toml -NoNewline -Encoding utf8
cargo build --release
if ($LASTEXITCODE) { throw 'cargo build failed' }

if (Test-Path publish) { Remove-Item publish -Recurse -Force }
New-Item -ItemType Directory publish | Out-Null
Copy-Item target\release\core-mod-manager.exe publish\

if (Test-Path Releases) { Remove-Item Releases -Recurse -Force }
# The previous release lets vpk make a small delta update (fails harmlessly for the first release).
vpk download github --repoUrl $repo --token $token
vpk pack --packId DPG.RogueCoreModManager --packVersion $Version --packDir publish --mainExe core-mod-manager.exe --packTitle 'DPG Rogue Core Mod Manager' --packAuthors DasLykia --icon "$env:TEMP\cmm-icon-build\app.ico"
if ($LASTEXITCODE) { throw 'vpk pack failed' }
vpk upload github --repoUrl $repo --token $token --publish --releaseName "v$Version" --tag "v$Version"
if ($LASTEXITCODE) { throw 'vpk upload failed' }

git add Cargo.toml Cargo.lock
git diff --cached --quiet
if ($LASTEXITCODE) { git commit -m "Release v$Version" }
git push
