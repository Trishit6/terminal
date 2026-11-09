<#>
.SYNOPSIS
    Installs KERNEXT on Windows.

.DESCRIPTION
    Downloads and installs the KERNEXT binary for Windows.
    Optionally adds the installation directory to the user's PATH.

.NOTES
    Requires PowerShell 5.1+ and Windows 10/11.
    Run with -Confirm:$false to skip the PATH confirmation prompt.
#>

param(
    [bool]$Confirm = $true,
    [string]$InstallDir = "$env:USERPROFILE\.local\bin",
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"

$VERSION = $Version
$REPO_URL = "https://github.com/Trishit6/terminal"
$BINARY_NAME = "kernext.exe"
$ARCHIVE_NAME = "kernext-x86_64-pc-windows-gnu.zip"
$ARCHIVE_URL = "$REPO_URL/releases/download/v$VERSION/$ARCHIVE_NAME"

# The banner and spinner are presentation only. They are skipped when output is
# redirected, when NO_COLOR is set, or when the host has no real console.
$UseUi = $true
try {
    if ([Console]::IsOutputRedirected -or $env:NO_COLOR -or $env:TERM -eq "dumb") {
        $UseUi = $false
    }
} catch {
    $UseUi = $false
}

function Write-Info { param($msg) if ($UseUi) { Write-Host "[INFO] $msg" -ForegroundColor Cyan } else { Write-Host "[INFO] $msg" } }
function Write-Success { param($msg) if ($UseUi) { Write-Host "[ OK ] $msg" -ForegroundColor Green } else { Write-Host "[ OK ] $msg" } }
function Write-Warning { param($msg) if ($UseUi) { Write-Host "[WARN] $msg" -ForegroundColor Yellow } else { Write-Host "[WARN] $msg" } }
function Write-Error { param($msg) if ($UseUi) { Write-Host "[FAIL] $msg" -ForegroundColor Red } else { Write-Host "[FAIL] $msg" } }

$LogoLines = @(
    "╦╔═╔═╗╦═╗╔╗╔╔═╗═╗╔╦",
    "╠╩╗║╣ ╠╦╝║║║║╣  ╠╩╗║",
    "╩ ╩╚═╝╩╚═╝╚╝╚═╝═╝ ╩"
)
$Palette = @("Cyan", "Blue", "Magenta", "DarkCyan", "DarkBlue", "DarkMagenta", "White")

function Show-Banner {
    Write-Host ""
    if ($UseUi) {
        $top = [Console]::CursorTop
        for ($frame = 0; $frame -lt 20; $frame++) {
            try { [Console]::SetCursorPosition(0, $top) } catch { }
            for ($row = 0; $row -lt $LogoLines.Count; $row++) {
                $color = $Palette[($row + $frame) % $Palette.Count]
                Write-Host $LogoLines[$row] -ForegroundColor $color
            }
            Start-Sleep -Milliseconds 30
        }
    } else {
        $LogoLines | ForEach-Object { Write-Host $_ }
    }

    if ($UseUi) {
        Write-Host ""
        Write-Host "KERNEXT " -NoNewline -ForegroundColor White
        Write-Host "v$VERSION" -ForegroundColor DarkGray
        Write-Host "The Next Core of Computing." -ForegroundColor Cyan
        Write-Host ""
    } else {
        Write-Host ""
        Write-Host "KERNEXT v$VERSION"
        Write-Host "The Next Core of Computing."
        Write-Host ""
    }
}

function Show-Stage {
    param([string]$Label)
    if ($UseUi) {
        foreach ($color in @("Cyan", "Blue", "Magenta")) {
            Write-Host "`r  $Label" -NoNewline -ForegroundColor $color
            Start-Sleep -Milliseconds 40
        }
        Write-Host "`r" -NoNewline
        Write-Host ""
    }
}

Show-Banner

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($isAdmin) {
    Write-Warning "Running as Administrator. Will install to $env:ProgramFiles\kernext."
    $InstallDir = "$env:ProgramFiles\kernext"
} else {
    Write-Info "Installing to user directory: $InstallDir"
}

Write-Info "Install directory: $InstallDir"

if (-not (Test-Path $InstallDir)) {
    try {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        Write-Info "Created directory: $InstallDir"
    } catch {
        Write-Error "Failed to create directory $InstallDir : $_"
        exit 1
    }
}

$tempDir = [System.IO.Path]::GetTempPath() + "kernext-install-" + [Guid]::NewGuid()
New-Item -ItemType Directory -Path $tempDir | Out-Null
$archivePath = Join-Path $tempDir "kernext.zip"

try {
    Show-Stage "Downloading $ARCHIVE_NAME"
    try {
        Invoke-WebRequest -Uri $ARCHIVE_URL -OutFile $archivePath -ErrorAction Stop
    } catch {
        Write-Error "Failed to download $ARCHIVE_URL : $_"
        Write-Info "You can manually download from: $REPO_URL/releases"
        exit 1
    }
    Write-Success "Downloaded KERNEXT v$VERSION."

    $checksumPath = "$archivePath.sha256"
    try {
        Invoke-WebRequest -Uri "$ARCHIVE_URL.sha256" -OutFile $checksumPath -ErrorAction Stop
        $expected = (Get-Content $checksumPath -TotalCount 1).Split(" ")[0].ToLower()
        $actual = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne $actual) {
            Write-Error "Checksum verification failed."
            exit 1
        }
        Write-Success "Checksum verified."
    } catch {
        if (Test-Path $checksumPath) {
            Write-Error "Checksum verification failed."
            exit 1
        }
        Write-Warning "No published checksum found; skipping verification."
    }

    Show-Stage "Extracting archive"
    Expand-Archive -Path $archivePath -DestinationPath $tempDir -Force

    $binaryPath = Join-Path $tempDir "kernext.exe"
    if (-not (Test-Path $binaryPath)) {
        $exeFiles = Get-ChildItem -Path $tempDir -Filter "*.exe" -Recurse
        if ($exeFiles.Count -gt 0) {
            $binaryPath = $exeFiles[0].FullName
        } else {
            Write-Error "Binary not found in archive."
            exit 1
        }
    }

    $destPath = Join-Path $InstallDir $BINARY_NAME
    Show-Stage "Installing to $destPath"
    Copy-Item $binaryPath -Destination $destPath -Force
    Write-Success "KERNEXT installed successfully."
    Write-Host "Build. Execute. Innovate." -ForegroundColor Cyan
    Write-Info "Run 'kernext' to start the shell."
    Write-Info "Run 'kernext --help' for usage information."

    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Warning "$InstallDir is not in your User PATH."
        if ($Confirm) {
            $addToPath = Read-Host "Add $InstallDir to your User PATH? (y/n)"
            if ($addToPath -eq 'y' -or $addToPath -eq 'Y') {
                $newPath = $currentPath + ";" + $InstallDir
                [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
                Write-Success "Added $InstallDir to User PATH. Restart your terminal for changes to take effect."
            }
        } else {
            Write-Warning "Add $InstallDir to your PATH manually: setx PATH `"$currentPath;$InstallDir`""
        }
    }

} finally {
    if (Test-Path $tempDir) {
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
