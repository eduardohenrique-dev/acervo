# Arquitetura

O Acervo é um workspace Cargo com quatro crates de domínio, o app Tauri 2 por cima, e a interface em React.

```
            ui (React + Tailwind)
                    │ comandos e eventos Tauri
            src-tauri (o app)
   ┌──────────┬─────┴─────┬──────────┐
classify    index       store       ops
categorias  MFT, USN,   snapshot    IFileOperation,
e regras    árvore,     e plano     política e log
            consultas   de abertura
```

Os crates não dependem do Tauri: dá para testar e rodar cada um sozinho (há exemplos de linha de comando em `index` e `store`).

## Crates

| Crate | O que tem |
| --- | --- |
| `classify` | as 10 categorias, a trie de caminhos conhecidos, as regras por nome e por extensão; ver [categorias](categorias.md) |
| `index` | leitura do volume e da MFT (`mft/`), varredura alternativa por pastas (`walk.rs`), árvore em memória mutável (`tree/`), consultas para as telas (`browse.rs`, `query.rs`) e o USN Journal (`usn/`) |
| `store` | formato do snapshot e `plan_start`, que decide se reaproveita ou reanalisa; ver [ao vivo e snapshot](ao-vivo-e-snapshot.md) |
| `ops` | operações pela `IFileOperation`, a política e o log; ver [operações e segurança](operacoes.md) |

## O app (`src-tauri`)

| Arquivo | Papel |
| --- | --- |
| `drives.rs` | lista os discos e a série de cada volume |
| `elevation.rs` | sabe se está rodando como administrador |
| `scan.rs` | análise de cada disco (MFT ou varredura por pastas) |
| `live.rs` | vigia do USN por disco e gravação periódica do snapshot |
| `state.rs` | o estado compartilhado: árvores, progresso, erros |
| `items.rs` | monta as linhas das telas Categoria e Arquivos |
| `queue.rs`, `operations.rs` | a fila de operações e os eventos de progresso |
| `secure.rs` | cria a pasta em `%ProgramData%` e aplica as permissões (pelo `icacls`) |
| `commands.rs` | os comandos chamados pela interface |

O build de release embute um manifesto (`app.manifest`) que pede administrador ao abrir.

## A interface (`ui`)

React 19 com TypeScript, Vite e Tailwind 4. As telas ficam em `screens/` (Visão geral, Categoria, Arquivos, Operações, Sistema) e os componentes em `components/` (barra de armazenamento, legenda, lista virtualizada, barra de ações, diálogo de confirmação, aviso de operação).

`lib/source.ts` escolhe a fonte dos dados: dentro do app, os comandos Tauri; fora dele, `lib/demo.ts`, com dois discos de exemplo, uma árvore de arquivos gerada e um "ao vivo" simulado. Por isso a interface abre no navegador sem compilar o Rust.

O sistema visual está em [design](design.md).

Voltar para a [documentação](README.md).
