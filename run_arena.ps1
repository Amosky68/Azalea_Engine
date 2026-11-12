# run_arena.ps1
$ErrorActionPreference = "Stop"

Write-Host "=== 1. Compilation Wasm ==="
cargo build -p azalea_bot --release --target wasm32-unknown-unknown
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERREUR DE COMPILATION"
    exit 1
}

Write-Host "=== 2. Lancement du duel Docker (600 parties) ==="
# Redirige stdout et stderr vers stats.txt pour tout capturer proprement
docker run --rm -v "${PWD}:/w" traversal-arena duel /w/target/wasm32-unknown-unknown/release/azalea_bot.wasm /w/target/wasm32-unknown-unknown/release/azalea_arena_2040.wasm --games 600 --fuel 700000000 --verbose *> stats.txt

# Affiche la ligne de résumé de l'arène et extrait le score
$summary = Select-String -Path stats.txt -Pattern "score\s+([0-9]+(?:\.[0-9]+)?)\s*%" | Select-Object -Last 1

if ($summary) {
    $winrate = $summary.Matches[0].Groups[1].Value
    Write-Host $summary.Line
    Write-Host "WINRATE: $winrate%"
} else {
    Write-Host "ERREUR : Impossible de lire le score dans stats.txt"
    Get-Content stats.txt -Tail 15
    exit 1
}