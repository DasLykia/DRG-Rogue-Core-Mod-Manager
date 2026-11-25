# Publishes a new version on GitHub with Velopack; installed apps then offer it as an update.
#   .\release.ps1 0.2.0
# Needs: Rust (GNU toolchain), vpk (dotnet tool install -g vpk), GitHub CLI logged in (gh auth login).
param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Continue'  # native tools write progress to stderr; failures are checked via $LASTEXITCODE
Set-Location $PSScriptRoot
$repo = 'https://github.com/DasLykia/DRG-Rogue-Core-Mod-Manager'
$token = gh auth token
if (-not $token) { throw 'GitHub CLI is not logged in (run: gh auth login)' }

# Commit any local changes (new files like assets\app.ico) before publishing.
git rm --cached --ignore-unmatch -q Release.bat MECHANICS.md mechanics.json SPEC.md  # local working files, not part of the repo
git add -A
git diff --cached --quiet
if ($LASTEXITCODE) { git commit -m "Prepare release v$Version" }

# First run: create the public repo and upload the code.
$remotes = @(git remote)
if ($remotes -notcontains 'origin') {
    gh repo create DasLykia/DRG-Rogue-Core-Mod-Manager --public --description 'DRG: RG Mod Manager - mod manager for Deep Rock Galactic: Rogue Core' --source . --remote origin --push
    if ($LASTEXITCODE) { throw 'creating the GitHub repo failed' }
} elseif ((git remote get-url origin) -match 'DPG-Rogue-Core-Mod-Manager') {
    # The repo was first published as DPG-...; rename it once.
    gh repo rename DRG-Rogue-Core-Mod-Manager --repo DasLykia/DPG-Rogue-Core-Mod-Manager --yes
    if ($LASTEXITCODE) {
        # Maybe it was already renamed on the website.
        gh repo view DasLykia/DRG-Rogue-Core-Mod-Manager --json name | Out-Null
        if ($LASTEXITCODE) { throw 'renaming the GitHub repo failed' }
    }
    git remote set-url origin https://github.com/DasLykia/DRG-Rogue-Core-Mod-Manager.git
    gh repo edit DasLykia/DRG-Rogue-Core-Mod-Manager --description 'DRG: RG Mod Manager - mod manager for Deep Rock Galactic: Rogue Core'
    # The releases made before the rename used the old app name (DPG.RogueCoreModManager): delete them and their tags.
    foreach ($tag in @(gh release list --repo DasLykia/DRG-Rogue-Core-Mod-Manager --limit 100 --json tagName --jq '.[].tagName')) {
        if ($tag) {
            Write-Host "Deleting old release $tag"
            gh release delete $tag --repo DasLykia/DRG-Rogue-Core-Mod-Manager --yes --cleanup-tag
            git tag -d $tag 2>$null | Out-Null
        }
    }
}

# The version shown in the app comes from Cargo.toml.
$toml = (Get-Content Cargo.toml -Raw) -replace '(?m)^version = ".*"', "version = `"$Version`""
[IO.File]::WriteAllText("$PSScriptRoot\Cargo.toml", $toml.TrimStart([char]0xFEFF), (New-Object Text.UTF8Encoding $false))
cargo build --release
if ($LASTEXITCODE) { throw 'cargo build failed' }

if (Test-Path publish) { Remove-Item publish -Recurse -Force }
New-Item -ItemType Directory publish | Out-Null
Copy-Item target\release\core-mod-manager.exe publish\

if (Test-Path Releases) { Remove-Item Releases -Recurse -Force }
# The previous release lets vpk make a small delta update (fails harmlessly for the first release).
vpk download github --repoUrl $repo --token $token
vpk pack --packId DRG.RogueCoreModManager --packVersion $Version --packDir publish --mainExe core-mod-manager.exe --packTitle 'DRG RG Mod Manager' --packAuthors DasLykia --icon "$PSScriptRoot\assets\app.ico"
if ($LASTEXITCODE) { throw 'vpk pack failed' }
vpk upload github --repoUrl $repo --token $token --publish --releaseName "v$Version" --tag "v$Version"
if ($LASTEXITCODE) { throw 'vpk upload failed' }

git add Cargo.toml Cargo.lock
git diff --cached --quiet
if ($LASTEXITCODE) { git commit -m "Release v$Version" }
git push
