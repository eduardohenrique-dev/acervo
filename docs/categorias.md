# Categorias

Todo arquivo do disco cai em exatamente uma das 10 categorias. A decisão fica no crate `classify` e é tomada **durante a descida pela árvore**: cada pasta herda a categoria da pasta de cima, a menos que uma regra diga outra coisa. Assim, classificar milhões de arquivos custa uma consulta por pasta, não por caminho completo.

## As três fontes de regra

### 1. Caminhos conhecidos

Caminhos absolutos desta máquina, descobertos ao abrir (pasta do Windows, Program Files, perfis de usuário, bibliotecas de jogos). Ficam numa trie de componentes de caminho.

| Caminho | Categoria |
| --- | --- |
| `C:\Windows`, `System Volume Information`, `Recovery`, `pagefile.sys`, `hiberfil.sys`, `swapfile.sys` | Sistema |
| `C:\Windows\Temp`, `SoftwareDistribution\Download`, `$Recycle.Bin`, `%TEMP%`, `INetCache`, `CrashDumps`, `D3DSCache`, `ProgramData\Microsoft\Windows\WER` | Cache e temporários |
| `Program Files`, `Program Files (x86)`, `%LOCALAPPDATA%\Programs` | Apps |
| bibliotecas da Steam (`libraryfolders.vdf`), jogos da Epic (manifestos `.item`) e da Riot (`RiotClientInstalls.json`), `XboxGames` | Jogos |
| `ProgramData`, `AppData\Roaming`, `AppData\Local`, `AppData\LocalLow` | Dados de apps |
| `Downloads` de cada perfil | Downloads |
| `Documentos` de cada perfil | Documentos (regra fraca) |

A regra mais específica vence: `Program Files (x86)\Steam\steamapps\common` é Jogos, mesmo dentro de Program Files.

### 2. Nomes de pasta

Valem em qualquer lugar do disco:

- **Desenvolvimento:** `node_modules`, `.venv`, `venv`, `.gradle`, `.m2`, `.cargo`, `.rustup`, `.npm`, `pnpm-store`, `.nuget`, uma pasta `target` ao lado de um `Cargo.toml`, e qualquer pasta que tenha um `.git` dentro (o repositório inteiro).
- **Cache e temporários:** `cache`, `code cache`, `gpucache`, `shadercache`, `cache2`, `dxcache`, `glcache`, quando estão dentro de Dados de apps.
- **Sistema:** pastas na raiz do disco que começam com `$`, exceto a Lixeira.

Dentro de Sistema e de Cache, nenhuma regra de nome se aplica: tudo continua na categoria de cima.

### 3. Extensão do arquivo

Imagens, vídeos e áudio viram **Mídia**; PDF, textos, planilhas, apresentações e e-books viram **Documentos**. A extensão só decide quando a pasta não tem regra forte.

## Forte e fraca

Uma regra **forte** vale para tudo o que está dentro. Uma regra **fraca** cede para a extensão: um vídeo dentro de Documentos conta como Mídia, mas o mesmo vídeo dentro de um jogo continua em Jogos.

## Outros

O que não casa com nenhuma regra fica em **Outros**: pastas criadas na raiz do disco, coleções de emuladores, backups soltos. Num segundo disco usado só para jogos fora de launcher, quase tudo cai aqui.

## Versão das regras

As regras padrão têm um número de versão. Quando ele muda, os snapshots antigos são reclassificados ao abrir, sem precisar de uma análise completa. Veja [ao vivo e snapshot](ao-vivo-e-snapshot.md).

Voltar para a [documentação](README.md).
