# Desenvolvimento

## Requisitos

- Windows 10 ou 11
- Rust (toolchain MSVC)
- Node 22 ou mais novo

## Comandos

```bash
npm install
npm --prefix ui install
npm run tauri dev      # sem administrador: varredura por pastas, sem ao vivo
npm run tauri build    # instalador em target/release/bundle/nsis
```

### Só a interface

```bash
npm --prefix ui run dev   # http://localhost:5199
```

Abre com discos de exemplo. Parâmetros úteis na URL:

| Parâmetro | Mostra |
| --- | --- |
| `?estado=analisando` | a análise em andamento |
| `?estado=falha` | um disco que falhou |
| `?estado=falha-op` | o primeiro item de cada operação falhando |

A cada 6 segundos, a pasta Downloads do C: de exemplo ganha ou perde 1,2 GB, para simular o ao vivo.

## Testes

```bash
cargo test --workspace
npm --prefix ui exec -- vitest --run
```

## Exemplos contra um disco de verdade

Num terminal de administrador:

```bash
# analisa o C: pela MFT e imprime os totais por categoria
cargo run -p acervo-index --release --example scan -- C

# analisa (ou carrega o snapshot de .\snap), acompanha o USN por 60 s
# imprimindo cada total que mudou, e grava o snapshot no fim
cargo run -p acervo-store --release --example watch -- C .\snap --for 60
```

## Ícones

O logo é `branding/logo.svg`. Para regenerar todos os ícones:

```bash
node branding/render.mjs
npx tauri icon app-icon.png -o src-tauri/icons
node branding/make-ico.mjs
```

Voltar para a [documentação](README.md).
