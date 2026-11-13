#!/usr/bin/env bash
# Confere a regra de dependência do workspace (ver CLAUDE.md).
# Falha se um crate do núcleo puxar I/O, framework ou uma camada de fora.
set -euo pipefail
cd "$(dirname "$0")/.."

FRAMEWORKS='tokio|axum|bevy|serde_json|tungstenite|tracing|crossbeam'
status=0

# check <crate> <regex de dependências proibidas>
check() {
    local crate="$1" forbidden="$2" found
    found=$(cargo tree -p "$crate" -e normal --prefix none 2>/dev/null \
        | tail -n +2 | awk '{print $1}' | sort -u | grep -E "^($forbidden)" || true)
    if [ -n "$found" ]; then
        echo "ERRO: $crate não pode depender de: $(echo "$found" | tr '\n' ' ')"
        status=1
    else
        echo "ok: $crate"
    fi
}

crate_name() {
    sed -n 's/^name *= *"\(.*\)"/\1/p' "$1/Cargo.toml" | head -n 1
}

# engine-*: nada de framework, nada de game-*, server ou client.
for dir in crates/engine/*/; do
    check "$(crate_name "$dir")" "$FRAMEWORKS|game-|server$|client$"
done

# game-sim: puro, sem framework nem serialização de fio.
check game-sim "$FRAMEWORKS|game-protocol|server$|client$"

# game-protocol: pode serializar, mas não conhece transporte nem render.
check game-protocol "tokio|axum|bevy|tungstenite|server$|client$"

exit $status
