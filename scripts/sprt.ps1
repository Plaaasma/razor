<#
.SYNOPSIS
Standardized SPRT between two engine binaries via fastchess (brief §5 protocol).
STC defaults: 8+0.08s, 1 thread, 16 MB hash, 16 concurrent, balanced book.

.EXAMPLE
.\sprt.ps1 -New ..\target\release\vendetta-new.exe -Base ..\target\release\vendetta-master.exe -Elo0 0 -Elo1 5 -Name tt-cutoffs
#>
param(
    [Parameter(Mandatory)][string]$New,
    [Parameter(Mandatory)][string]$Base,
    [Parameter(Mandatory)][string]$Name,
    [double]$Elo0 = 0,
    [double]$Elo1 = 5,
    [string]$TC = "8+0.08",
    [int]$HashMB = 16,
    [int]$Concurrency = 16,
    [string]$Book = "H:\RazorBot\books\8moves_v3.pgn",
    [switch]$Detached
)

$fastchess = "H:\RazorBot\tools\fastchess\fastchess-windows-x86-64\fastchess.exe"
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outDir = "H:\RazorBot\matches"
New-Item -ItemType Directory -Force $outDir | Out-Null
$pgnOut = Join-Path $outDir "sprt-$Name-$stamp.pgn"

$fcArgs = @(
    "-engine", "cmd=$New", "name=new",
    "-engine", "cmd=$Base", "name=base",
    "-each", "tc=$TC", "option.Hash=$HashMB", "threads=1",
    "-openings", "file=$Book", "format=pgn", "order=random",
    "-repeat", "-games", "2", "-rounds", "100000",
    "-sprt", "elo0=$Elo0", "elo1=$Elo1", "alpha=0.05", "beta=0.05",
    "-concurrency", "$Concurrency",
    "-ratinginterval", "50",
    "-recover",
    "-pgnout", "file=$pgnOut"
) -join " "

if ($Detached) {
    & "$PSScriptRoot\run_detached.ps1" -Name "sprt-$Name" -Exe $fastchess -Args $fcArgs
} else {
    Write-Output "sprt '$Name': elo0=$Elo0 elo1=$Elo1 tc=$TC"
    & $fastchess $fcArgs.Split(" ")
}
