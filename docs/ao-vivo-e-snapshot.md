# Ao vivo e snapshot

## A primeira análise: MFT

Num volume NTFS, a Master File Table guarda um registro para cada arquivo e pasta: nome, pasta-mãe e tamanho. O crate `index` abre o volume (`\\.\C:`), lê a MFT em blocos grandes e monta a árvore em memória a partir desses registros, sem chamar a API de listar pastas. Um disco de 1 TB com milhões de arquivos leva segundos.

Sem administrador (por exemplo em `npm run tauri dev`), o app não consegue abrir o volume e cai na varredura por pastas: funciona, mas é bem mais lenta e não entra nas áreas protegidas.

## Ao vivo: USN Journal

O NTFS mantém um diário de mudanças, o USN Journal: cada criação, exclusão, renomeação ou mudança de tamanho vira um registro com um número sequencial (USN). Com o disco analisado, o Acervo acompanha esse diário a partir do último número que leu e aplica cada mudança na árvore em memória. A barra muda em menos de um segundo.

As mudanças chegam em lote: o app junta os registros de um intervalo curto, atualiza a árvore de uma vez e só então avisa a interface.

## Snapshot

Ao fechar (e a cada 10 minutos), a árvore de cada disco é gravada em `%ProgramData%\Acervo\index-<série-do-volume>.bin`, junto com o último USN lido, a identidade do diário e a versão das regras de categoria.

Ao abrir, o crate `store` decide o que fazer (`plan_start`):

| Situação | O que acontece |
| --- | --- |
| snapshot válido e o diário cobre o tempo fechado | carrega o snapshot e reaplica só o que mudou |
| as regras de categoria mudaram de versão | carrega e reclassifica, sem reler o disco |
| arquivo não existe, está corrompido ou é de outra versão | análise completa |
| o diário foi apagado, recriado, ou ficou curto demais para o tempo fechado | análise completa |
| "Tentar de novo" na interface | análise completa |

O número de série do volume no nome do arquivo evita misturar discos que trocaram de letra.

## Pasta protegida

A pasta `%ProgramData%\Acervo` é criada com permissão só para o sistema e para administradores. Como o app roda elevado e confia no snapshot, um programa comum não pode trocar esse arquivo para enganá-lo. As permissões são aplicadas ao abrir, cortando a herança e removendo os grupos comuns. Se isso falhar, o app segue sem snapshot, em vez de gravar numa pasta que outros poderiam alterar.

Voltar para a [documentação](README.md).
