[CmdletBinding()]
param(
  [string]$ProcessAgentUrl = $env:PROCESS_CPU_AGENT_URL,
  [string]$WindowsExporterVersion = "0.25.1",
  [int]$WindowsExporterPort = 31415,
  [int]$ProcessAgentPort = 31416
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptRoot "..")
$LibDir = Join-Path $RepoRoot "lib"
$DefaultAgentSrc = Join-Path $LibDir "process-cpu-agent.exe"
$InstallRoot = "C:\Program Files\prometheus"
$WindowsInstallerPath = Join-Path $InstallRoot "windows_exporter.msi"
$ProcessAgentDir = Join-Path $InstallRoot "process-cpu-agent"
$ProcessAgentExe = Join-Path $ProcessAgentDir "process-cpu-agent.exe"
$ConfigPath = Join-Path $ProcessAgentDir "config.yaml"

function Write-Info {
  param([string]$Message)
  Write-Host "[+] $Message"
}

function Write-Fatal {
  param([string]$Message)
  Write-Error $Message
  exit 1
}

function Assert-Admin {
  $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltinRole]::Administrator)) {
    Write-Fatal "Run this script from an elevated PowerShell session."
  }
}

function Get-WindowsExporterArch {
  if ([Environment]::Is64BitOperatingSystem) {
    return "amd64"
  }
  return "386"
}

function Invoke-Download {
  param(
    [string]$Uri,
    [string]$OutFile
  )
  Write-Info "Downloading $Uri"
  Invoke-WebRequest -Uri $Uri -OutFile $OutFile -UseBasicParsing
}

function Install-WindowsExporter {
  New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null
  $arch = Get-WindowsExporterArch
  $uri = "https://github.com/prometheus-community/windows_exporter/releases/download/v$WindowsExporterVersion/windows_exporter-$WindowsExporterVersion-$arch.msi"
  Invoke-Download -Uri $uri -OutFile $WindowsInstallerPath

  Write-Info "Installing windows_exporter $WindowsExporterVersion"
  $arguments = @(
    "/i", $WindowsInstallerPath,
    "/quiet",
    "/norestart",
    "INSTALLDIR=$InstallRoot",
    "LISTEN_PORT=$WindowsExporterPort",
    "ENABLED_COLLECTORS=cpu,cs,logical_disk,net,os,service,system,textfile,process,memory"
  )
  $process = Start-Process -FilePath msiexec.exe -ArgumentList $arguments -Wait -PassThru
  if ($process.ExitCode -ne 0) {
    Write-Fatal "msiexec failed with exit code $($process.ExitCode)"
  }

  Start-Process -FilePath sc.exe -ArgumentList "config", "windows_exporter", "start= auto" -NoNewWindow -Wait | Out-Null
  Start-Process -FilePath sc.exe -ArgumentList "start", "windows_exporter" -NoNewWindow -Wait | Out-Null
  Write-Info "windows_exporter service configured"
}

function Install-ProcessAgent {
  New-Item -ItemType Directory -Path $ProcessAgentDir -Force | Out-Null

  if ($ProcessAgentUrl) {
    Invoke-Download -Uri $ProcessAgentUrl -OutFile $ProcessAgentExe
  } elseif (Test-Path $DefaultAgentSrc) {
    Write-Info "Copying bundled process-cpu-agent.exe"
    Copy-Item -Path $DefaultAgentSrc -Destination $ProcessAgentExe -Force
  } else {
    Write-Fatal "Process CPU Agent source missing. Provide --ProcessAgentUrl."
  }

  Write-Info "Writing process agent config"
  @(
    "# Process CPU Agent Configuration",
    "port: $ProcessAgentPort",
    "interval: 15",
    "max_processes: 100"
  ) | Set-Content -Encoding UTF8 -Path $ConfigPath

  $existing = Get-Service -Name "ProcessCpuAgent" -ErrorAction SilentlyContinue
  if ($existing) {
    Write-Info "Updating existing ProcessCpuAgent service"
    Start-Process -FilePath sc.exe -ArgumentList "stop", "ProcessCpuAgent" -NoNewWindow -Wait | Out-Null
    Start-Process -FilePath sc.exe -ArgumentList "delete", "ProcessCpuAgent" -NoNewWindow -Wait | Out-Null
  }

  $binPath = "\"$ProcessAgentExe\" --port $ProcessAgentPort"
  Start-Process -FilePath sc.exe -ArgumentList "create", "ProcessCpuAgent", "binPath= $binPath", "DisplayName= Process CPU Agent for Prometheus", "start= auto" -NoNewWindow -Wait | Out-Null
  Start-Process -FilePath sc.exe -ArgumentList "start", "ProcessCpuAgent" -NoNewWindow -Wait | Out-Null
  Write-Info "ProcessCpuAgent service created"
}

function Print-NextSteps {
  Write-Host "Next steps:"
  Write-Host "  - Verify windows_exporter via http://localhost:$WindowsExporterPort/metrics"
  Write-Host "  - Verify Process CPU Agent via http://localhost:$ProcessAgentPort/metrics"
  Write-Host "  - Monitor services with sc query windows_exporter and sc query ProcessCpuAgent"
}

Assert-Admin
Install-WindowsExporter
Install-ProcessAgent
Print-NextSteps
