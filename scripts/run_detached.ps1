<#
.SYNOPSIS
Launch a long-running command detached from this session, with logging and a PID file.
Replacement for tmux on this Windows host (see STATE.md "Environment Deviations").

.EXAMPLE
.\run_detached.ps1 -Name sprt-tt -Exe fastchess.exe -Args '-engine ... -sprt ...'
#>
param(
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][string]$Exe,
    [string]$Args = "",
    [string]$WorkDir = "H:\RazorBot"
)

$logDir = "H:\RazorBot\logs"
New-Item -ItemType Directory -Force $logDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$log = Join-Path $logDir "$Name-$stamp.log"
$errLog = Join-Path $logDir "$Name-$stamp.err.log"

$p = Start-Process -FilePath $Exe -ArgumentList $Args -WorkingDirectory $WorkDir `
    -RedirectStandardOutput $log -RedirectStandardError $errLog -PassThru -WindowStyle Hidden

@{ name = $Name; pid = $p.Id; exe = $Exe; args = $Args; log = $log; started = $stamp } |
    ConvertTo-Json | Out-File -Encoding utf8 (Join-Path $logDir "$Name-$stamp.pid.json")

Write-Output "started '$Name' pid=$($p.Id) log=$log"
