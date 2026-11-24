# Instalação e uso

## Instalar

1. Gere o instalador com `npm run tauri build`.
2. Abra `Acervo_x.y.z_x64-setup.exe` pelo Explorer, com duplo clique.
3. Ao abrir o Acervo, o Windows pede permissão de administrador: é o que permite ler a MFT e o USN Journal.

> Rode o instalador pelo Explorer, não de dentro de um terminal aberto por um app empacotado (MSIX). Nesses terminais o Windows redireciona o `AppData` para a pasta privada do pacote, e o Explorer não enxerga a instalação.

## Visão geral

Cada disco aparece com uma barra dividida por categoria, a legenda com o tamanho de cada uma e o espaço livre. O selo **Ao vivo** indica que o disco está sendo acompanhado pelo USN Journal. A primeira análise de um disco leva alguns segundos; as seguintes partem do snapshot.

Clique numa categoria (na legenda ou na barra) para entrar nela. O **Sistema** abre a tela Sistema, que detalha o que o Windows ocupa (arquivo de paginação, hibernação, pontos de restauração, a pasta Windows) e não tem ações.

## Dentro de uma categoria

As pastas e arquivos daquela categoria no disco, do maior para o menor. Duplo clique (ou a seta) entra na pasta; a trilha no alto volta.

## Arquivos

Todos os arquivos do disco numa lista só, do maior para o menor. Clicar de novo em **Tamanho** inverte a ordem. Dá para buscar pelo nome e filtrar por categoria, tamanho e "sem mexer há mais de". A lista é virtualizada: aguenta milhões de linhas sem travar.

## Ações

Selecione itens com a caixa da linha e use a barra que aparece embaixo:

| Ação | Atalho | Confirmação |
| --- | --- | --- |
| Lixeira | Delete | pede confirmação; dá para restaurar |
| Apagar de vez | Shift+Delete | só libera depois de marcar "Entendo que…" |
| Mover, copiar | — | escolhe a pasta de destino |
| Renomear | — | um item por vez |
| Abrir no Explorer | — | — |

O que pode ou não ser mexido está em [operações e segurança](operacoes.md).

## Operações

As ações entram numa fila e rodam uma por vez. Um aviso no canto mostra o progresso e o resultado. A tela **Operações** guarda o que foi feito na sessão e, quando algo falha, o motivo de cada item.

## Onde ficam os arquivos do Acervo

Tudo em `%ProgramData%\Acervo`, numa pasta acessível só ao sistema e a administradores:

- `index-<série-do-volume>.bin`: o snapshot de cada disco;
- `log\operacoes-AAAA-MM.log`: o registro das operações.

Para forçar uma análise completa de um disco, apague o `index-*.bin` dele com o app fechado, ou use "Tentar de novo".

Voltar para a [documentação](README.md).
