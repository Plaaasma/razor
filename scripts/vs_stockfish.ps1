<#
.SYNOPSIS
The benchmark that defines success (brief §7). Vendetta vs pinned Stockfish 18.
500+ game pairs, UHO unbalanced book, both sides of each opening.
TC 60+0.6, 8 threads each, 256 MB hash, Syzygy adjudication, concurrency 2.

MUST run on an otherwise idle machine (brief §8) — check Task Manager first.

.EXAMPLE
.\vs_stockfish.ps1 -Engine ..\target\release\vendetta.exe -Pairs 500 -Detached
#>
param(
    [Parameter(Mandatory)][string]$Engine,
    [int]$Pairs = 500,
    [string]$TC = "60+0.6",
    [int]$Threads = 8,
    [int]$HashMB = 256,
    [switch]$Detached
)

$fastchess = "H:\RazorBot\tools\fastchess\fastchess-windows-x86-64\fastchess.exe"
$stockfish = "H:\RazorBot\tools\stockfish\stockfish\stockfish-windows-x86-64-bmi2.exe"
$book = "H:\RazorBot\books\UHO_XXL_2022_+120_+149.pgn"
$syzygy = "H:\RazorBot\syzygy"
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outDir = "H:\RazorBot\matches"
New-Item -ItemType Directory -Force $outDir | Out-Null
$pgnOut = Join-Path $outDir "vs-sf-$stamp.pgn"
$epdLog = Join-Path $outDir "vs-sf-$stamp.config.json"

$fcArgs = @(
    "-engine", "cmd=$Engine", "name=vendetta",
    "-engine", "cmd=$stockfish", "name=sf18",
    "-each", "tc=$TC", "option.Hash=$HashMB", "option.Threads=$Threads",
    "-openings", "file=$book", "format=pgn", "order=random",
    "-repeat", "-games", "2", "-rounds", "$Pairs",
    "-concurrency", "2",
    "-tb", $syzygy,
    "-ratinginterval", "10",
    "-recover",
    "-pgnout", "file=$pgnOut"
) -join " "

@{ engine = $Engine; pairs = $Pairs; tc = $TC; threads = $Threads; hash = $HashMB; book = $book; pgn = $pgnOut } |
    ConvertTo-Json | Out-File -Encoding utf8 $epdLog

if ($Detached) {
    & "$PSScriptRoot\run_detached.ps1" -Name "vs-sf" -Exe $fastchess -Args $fcArgs
} else {
    & $fastchess $fcArgs.Split(" ")
}

# Post-match: analyze $pgnOut for style metrics (§6) — win rate, decisive rate,
# comeback wins, sacrifice counter, eval volatility. Analyzer script comes with Phase 4.
