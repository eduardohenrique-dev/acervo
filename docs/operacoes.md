# Operações e segurança

Um app que roda como administrador e apaga arquivos precisa de travas. As do Acervo ficam no crate `ops`, separadas da interface: a interface só pede, e a política decide.

## O mecanismo

Toda ação passa pela `IFileOperation` do Windows, a mesma que o Explorer usa. Com isso vêm de graça a Lixeira de verdade, os nomes de conflito ("arquivo (2)") e o comportamento esperado com arquivos em uso.

As operações entram numa fila e rodam uma por vez, numa thread própria. A interface recebe progresso e resultado item a item.

## O que não pode ser mexido

A política (`policy.rs`) recusa, com o motivo dito na tela:

- tudo o que está na categoria **Sistema** ("O Windows precisa disso para funcionar");
- a raiz de um disco;
- os atalhos de pasta do Windows (junções de sistema como "Dados de Aplicativos");
- as pastas do próprio Acervo, e qualquer pasta que as contenha;
- mover ou copiar para um destino nessas mesmas áreas.

Abrir no Explorer é sempre permitido.

## O que pede confirmação a mais

Mandar para a Lixeira, apagar de vez ou mover algo das categorias **Apps** ou **Dados de apps** mostra um aviso antes: "Isto pertence a um programa. Se ele ainda estiver instalado, o certo é desinstalar."

**Apagar de vez** só libera o botão depois de marcar "Entendo que…". A Lixeira é o padrão.

## Registro

Cada operação grava em `%ProgramData%\Acervo\log\operacoes-AAAA-MM.log`: quando, o quê, de onde, para onde e o resultado de cada item. A tela Operações mostra o mesmo para a sessão atual.

## Ao vivo depois da operação

Num disco ao vivo, quem atualiza a árvore depois de apagar ou mover é o USN Journal, igual a qualquer outra mudança no disco. Assim a barra mostra o que de fato aconteceu, inclusive quando parte da operação falha. Num disco sem ao vivo (modo desenvolvimento, sem administrador), o app atualiza as pastas envolvidas ao fim da operação.

Voltar para a [documentação](README.md).
