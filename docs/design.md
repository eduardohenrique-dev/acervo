---
name: Acervo
description: Armazenamento do PC como cápsulas de vidro cheias de líquido colorido, em liquid glass próprio sobre o Acrylic do Windows 11.
colors:
  accent: "#0a6cff"
  accent-dark: "#3b8cff"
  danger: "#d70015"
  danger-dark: "#ff6961"
  text-primary: "#111214"
  text-secondary: "#55585f"
  text-tertiary: "#6e727a"
  text-primary-dark: "#f5f5f7"
  text-secondary-dark: "#b4b6bd"
  text-tertiary-dark: "#95989f"
  ground: "rgba(244, 244, 247, 0.72)"
  ground-dark: "rgba(20, 20, 24, 0.62)"
  glass-fill: "rgba(255, 255, 255, 0.58)"
  glass-fill-dark: "rgba(44, 44, 50, 0.52)"
  glass-fill-strong: "rgba(255, 255, 255, 0.42)"
  glass-fill-strong-dark: "rgba(36, 36, 42, 0.5)"
  capsule-track: "rgba(15, 20, 40, 0.07)"
  capsule-track-dark: "rgba(0, 0, 0, 0.28)"
  separator: "rgba(15, 20, 40, 0.08)"
  cat-apps: "#007aff"
  cat-jogos: "#af52de"
  cat-dev: "#5856d6"
  cat-midia: "#ff2d55"
  cat-documentos: "#ff9500"
  cat-downloads: "#f5c400"
  cat-cache: "#32ade6"
  cat-dados-apps: "#34c759"
  cat-outros: "#a2845e"
  cat-sistema: "#8e8e93"
typography:
  large-title:
    fontFamily: "Inter Variable, Inter, system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 700
    lineHeight: 1.25
    letterSpacing: "-0.022em"
  title:
    fontFamily: "Inter Variable, Inter, system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 600
    letterSpacing: "-0.012em"
  body:
    fontFamily: "Inter Variable, Inter, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 400
  label:
    fontFamily: "Inter Variable, Inter, system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 600
  caption:
    fontFamily: "Inter Variable, Inter, system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 400
rounded:
  control: "6px"
  panel: "8px"
  capsule: "5px"
  chip: "4px"
  window: "10px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "20px"
  "6": "24px"
  "8": "32px"
  "10": "40px"
components:
  glass-panel:
    backgroundColor: "{colors.glass-fill}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.panel}"
    padding: "16px 20px"
  sidebar:
    backgroundColor: "{colors.glass-fill-strong}"
    rounded: "{rounded.panel}"
    padding: "8px"
    width: "188px"
  nav-item:
    textColor: "{colors.text-secondary}"
    typography: "{typography.body}"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "36px"
  nav-item-active:
    backgroundColor: "rgba(15, 20, 40, 0.07)"
    textColor: "{colors.text-primary}"
  capsule:
    backgroundColor: "{colors.capsule-track}"
    rounded: "{rounded.capsule}"
    padding: "2px"
    height: "16px"
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "#ffffff"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "28px"
  legend-item:
    textColor: "{colors.text-primary}"
    typography: "{typography.caption}"
    rounded: "7px"
    height: "24px"
  window-control:
    textColor: "{colors.text-secondary}"
    rounded: "8px"
    width: "40px"
    height: "28px"
---

# Design System: Acervo

## Overview

**Creative North Star: "A Cápsula de Vidro"**

O armazenamento é um tubo de vidro cheio de líquido colorido: cada disco é uma cápsula, e as categorias são camadas de vidro tingido que se leem em um olhar. A janela é translúcida sobre o Acrylic do Windows 11, os painéis são vidro com aro especular de 1px e brilho superior, e as tintas são as cores de sistema da Apple. A identidade é própria no estilo Apple, não Fluent: do Windows ficam só os controles de janela e o material por trás.

A densidade é calma e a hierarquia vem de tamanho e peso, não de cor: título grande, um painel por disco, números tabulares. A cor existe quase só nas categorias; o resto é neutro e vidro. O tema segue o Windows (claro e escuro) com os mesmos nomes de token.

Recusa o painel cinza de tabelas e gráficos de pizza dos limpadores de disco.

**Key Characteristics:**
- Vidro translúcido (fill + blur 28px + saturação 180%) sobre um chão que tinge o Acrylic para garantir contraste.
- A cápsula é a assinatura: tubo de 16px, líquido que termina em menisco, reflexo especular que segue o cursor, segmentos que escoam com mola.
- Dez tintas de categoria, sempre acompanhadas de rótulo e tamanho.
- Inter variável, números tabulares em todo valor de tamanho.
- Cantos contínuos e contidos, na escala da coletânea Quintal (janela 10px, painel 8px, controle 6px, etiqueta 4px).

## Colors

Neutros frios e translúcidos como base; a cor vive nas categorias e em um único azul de ação.

### Primary
- **Azul de Ação** (#0a6cff claro / #3b8cff escuro): botão "Tentar de novo", foco visível, ícone do item ativo da barra lateral, varredura da análise. Nunca decora superfície.

### Secondary
- **Vermelho de Falha** (#d70015 claro / #ff6961 escuro): só o fundo tingido a 8% do aviso de falha da análise.

### Tertiary (tintas das categorias, variante clara; no escuro usam as variantes mais vivas da Apple)
- **Azul Apps** (#007aff), **Roxo Jogos** (#af52de), **Índigo Desenvolvimento** (#5856d6), **Rosa Mídia** (#ff2d55), **Laranja Documentos** (#ff9500), **Amarelo Downloads** (#f5c400), **Ciano Cache** (#32ade6), **Verde Dados de apps** (#34c759), **Areia Outros** (#a2845e), **Cinza Sistema** (#8e8e93). No escuro: #0a84ff, #bf5af2, #5e5ce6, #ff375f, #ff9f0a, #ffd60a, #64d2ff, #30d158, #ac8e68, #8e8e93.

### Neutral
- **Tinta Principal** (#111214 / #f5f5f7), **Tinta Secundária** (#55585f / #b4b6bd, 7:1 sobre o vidro claro), **Tinta Terciária** (#6e727a / #95989f, 4,6:1).
- **Chão da Janela** (rgba(244,244,247,.72) / rgba(20,20,24,.62)): tinge o Acrylic.
- **Vidro** (fill 58% branco / 52% grafite) e **Vidro Forte** (42% / 50%, barra lateral e avisos).
- **Tubo Vazio** (rgba(15,20,40,.07) / rgba(0,0,0,.28)) e **Separador** (rgba(15,20,40,.08)).

### Named Rules
**The Tinta de Vidro Rule.** Categoria é tinta de vidro, nunca cor chapada: o segmento usa a cor a 82% com gradiente (claro no topo, denso embaixo).
**The Rótulo Sempre Rule.** Nenhuma categoria é identificada só por cor; todo ponto colorido vem com nome e tamanho.
**The Um Azul Rule.** O azul de ação é a única cor fora das categorias; ação e categoria Apps se distinguem por contexto, não por mistura.

## Typography

**Display/Body/Label Font:** Inter Variable (com Inter, system-ui, sans-serif), recursos cv11 e ss03.

**Character:** Uma única família, limpa e neutra, no peso do "large title" do iOS; os números tabulares (`.num`) dão régua aos tamanhos em GB.

### Hierarchy
- **Large title** (700, 2rem/32px, line-height 1.25, -0.022em): título da tela ("Armazenamento", "Sistema").
- **Title** (600, 1.0625rem/17px, -0.012em): nome do disco.
- **Body** (400, 0.875rem/14px): "X de Y usados", itens da barra lateral, descrições; textos corridos até 56ch.
- **Label** (600, 13px): botões, abas de disco, nome na barra de título (13px, -0.01em).
- **Caption** (400, 0.75rem/12px): legenda, "livres", dicas, avisos de análise.

### Named Rules
**The Número Tabular Rule.** Todo valor de tamanho ou porcentagem usa `.num` (tabular-nums); formato pt-BR em base 1024 ("53,1 GB", "< 1%").

## Layout

Janela em grade de duas linhas: barra de título de 36px e corpo. No corpo, barra lateral de 188px flutuante (margem 8px, itens de 32px) e área principal rolável com 24px de padding lateral e embaixo, conteúdo centrado até 960px, painéis empilhados com 16px de intervalo. Densidade de app de produtividade: corpo 13px, legendas 12px, linhas de lista de 30px. Espaço em base 4 (4 a 40px). Painel de disco: 20px laterais, 16px verticais; cabeçalho com nome à esquerda e resumo à direita, cápsula em largura total, legenda em linha que quebra, "livres" alinhado à direita. A tela Sistema repete a moldura; cada linha é grade de três colunas (rótulo e dica, barra de 8px de 120 a 240px, valor de 88px) separada por filetes.

## Elevation & Depth

Profundidade híbrida: vidro translúcido empilhado sobre o Acrylic, com sombra suave e borda especular. Não há sombra dura.

### Shadow Vocabulary
- **Vidro** (`inset 0 1px 0 var(--glass-shine), inset 0 0 0 1px (borda a 40%), 0 1px 2px rgba(20,24,40,.06), 0 10px 30px -8px rgba(20,24,40,.14)`; no escuro `0 1px 2px rgba(0,0,0,.3), 0 14px 36px -10px rgba(0,0,0,.55)`): painéis e barra lateral.
- **Tubo** (`inset 0 1px 0 rgba(255,255,255,.35), inset 0 -1px 2px rgba(0,0,0,.22), inset 0 0 0 1px var(--glass-edge-low)`): cavidade da cápsula.
- **Botão de ação** (`0 1px 2px rgba(0,0,0,.2), inset 0 1px 0 rgba(255,255,255,.25)`).

### Named Rules
**The Aro Especular Rule.** Toda superfície de vidro tem brilho interno de 1px no topo e borda baixa; sem isso é só um retângulo translúcido.
**The Reflexo Vivo Rule.** O brilho que segue o cursor (`--mx`, soft-light) só existe sobre a cápsula, só enquanto o cursor está nela.

## Shapes

Cantos contínuos na escala da coletânea: janela 10px, painéis 8px, controles 6px, etiquetas 4px, cápsula 5px. A cápsula tem 16px de altura com 2px de folga interna; o líquido termina arredondado (menisco) antes do tubo vazio. Pontos de categoria são círculos de 10px. Segmentos são separados por um filete branco especular de 1px, não por espaço.

## Components

### Glass panel (DiskCard, painel do Sistema)
Vidro, raio 8px, 20px de padding horizontal; entra com fade e subida de 8px (0,4s, ease-out expo). Cabeçalho com nome (Title) e resumo; o resumo troca por "categoria · tamanho · % do disco" quando um segmento está em foco.

### Storage capsule
- Líquido anima largura e segmentos com mola (rigidez 170, amortecimento 26, massa 0,9).
- Com um segmento em foco, os demais perdem saturação e opacidade (0,55).
- Faixa de brilho fixa de 6px no topo; brilho radial segue o cursor.
- Em análise: líquido azul a 70% com varredura de luz (1,6s, desligada em "reduzir movimento") e texto de fase ("Lendo o índice do disco" / "Percorrendo as pastas") com porcentagem.
- A versão fina (8px) mostra a participação de cada item na tela Sistema.

### Category legend
Linha com ponto de 10px, rótulo e tamanho terciário, altura 28px, raio 4px. Item em foco destaca seu segmento; os outros caem a 45% de opacidade. Só o Sistema é botão (com chevron e hover); os demais são informativos com tabindex.

### Sidebar and navigation
Painel de vidro forte, itens de 32px, raio 6px. Ativo: fundo `selected`, texto primário, peso 600, ícone em azul de ação. Inativo: texto secundário, hover sutil. Ícones de traço de 16px (1,5px, pontas redondas).

### Segmented control (abas de disco)
Vidro forte, raio 11px; a aba ativa é uma pílula de vidro que desliza com mola (`layoutId`). Aparece só com mais de um disco pronto.

### Buttons
Primário: azul de ação, texto branco 13px/600, 32px de altura, raio 10px, ícone de 14px; hover `brightness(1.1)`, active 0,95. Controles de janela: 44x32px, raio 8px, hover sutil; fechar vira vermelho #e81123 (convenção do Windows).

### Notices
Aviso informativo: vidro forte, raio 14px. Falha: fundo do vermelho de falha a 8%, texto primário e botão "Tentar de novo". Selo "Números aproximados": pílula com fundo `hover`, só na exceção sem administrador.

## Do's and Don'ts

### Do:
- **Do** deixar a cápsula ser o herói de cada disco: largura total, 16px, líquido de categorias com menisco.
- **Do** tingir o vidro pela categoria usando `--cat-*` e acompanhar a cor sempre de rótulo e tamanho.
- **Do** usar `.num` e o formato pt-BR de `formatBytes`/`formatPercent` em todo número.
- **Do** manter o foco visível (2px azul, offset 2px) e respeitar "reduzir movimento" (MotionConfig `user`).
- **Do** usar raios 22/10/pílula e espaço base 4.
- **Do** manter só o Sistema como categoria clicável enquanto não houver ações para as outras.

### Don't:
- **Don't** usar painel cinza de tabelas ou gráficos de pizza para mostrar armazenamento.
- **Don't** aplicar a identidade Fluent/Windows 11 aos painéis; só controles de janela e Acrylic.
- **Don't** usar uma cor de categoria como decoração fora da cápsula, legenda e pontos.
- **Don't** usar sombra dura ou borda opaca em superfície de vidro.
- **Don't** remover o reflexo especular ou o aro de 1px das superfícies de vidro.
- **Don't** pôr mais de uma cor de ação além do azul.

### Not canonized (defeitos que o build carrega)
- Raios soltos fora da escala de tokens (14px nos avisos, 8/9/11px em controles) e tamanhos 13px fora da rampa: deriva a normalizar, não regra.
- Vermelho #e81123 do botão fechar é literal hard-coded, não token.
