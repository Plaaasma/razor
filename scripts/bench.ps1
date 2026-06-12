<#
.SYNOPSIS
Run the engine's `bench` command and report signature node count.
Used to verify non-functional changes (node count must be identical) and track speed.

.EXAMPLE
.\bench.ps1 -Engine ..\target\release\vendetta.exe
#>
param(
    [Parameter(Mandatory)][string]$Engine
)

$out = & $Engine bench 2>&1
$out | Select-Object -Last 5
$nodes = ($out | Select-String -Pattern "^Nodes searched\s*:\s*(\d+)").Matches.Groups[1].Value
if ($nodes) { Write-Output "BENCH_NODES=$nodes" } else { Write-Output "BENCH_NODES=PARSE_FAILED" }
