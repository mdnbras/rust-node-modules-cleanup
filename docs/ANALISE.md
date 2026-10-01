# Análise e migração para Rust

Referência: [sebastianekstrom/node-modules-cleanup](https://github.com/sebastianekstrom/node-modules-cleanup), commit `74e099ee05a57c683d9e7a1887b806f9f7a08d86`, analisado em 01/10/2026.

## Fluxo original

1. `src/main.ts` interpreta argumentos e resolve o primeiro argumento como caminho.
2. `findNodeModulesFolders.ts` percorre diretórios, ignora nomes iniciados por ponto e interrompe a descida quando encontra `node_modules`. Erros de leitura são ignorados.
3. `calculateSizeOfNodeModulesDirs.ts` mede grupos de até 10 pastas. `getDirectorySize.ts` soma tamanhos usando operações assíncronas; usa `stat` nos itens não classificados como diretório.
4. Exibe uma tabela, pede confirmação, ou pula a pergunta com `--skip-confirmation`.
5. `deleteFolders.ts` apaga em grupos de até 5 com `fs.promises.rm`, recursivamente.
6. `--dry` simula progresso com atrasos. O resumo compartilhado com a exclusão real pode dizer que apagou arquivos mesmo na simulação.

## Correspondência

| Original | Implementação Rust |
| --- | --- |
| Busca recursiva e subdiretórios ocultos ignorados | Mesma seleção, com pilha iterativa |
| Não listar `node_modules` aninhado duas vezes | Preservado |
| Tabela de tamanho e total | Lista ordenada com unidades binárias e total |
| `--skip-confirmation` | Preservado, alias `-y` |
| `--dry` | Preservado; sem pergunta, atrasos artificiais ou mensagem de exclusão |
| `--help`, `--h`, `--version`, `--v` | Preservados; também `-h` e `-V` |
| Remoção em lotes de 5 | Padrão 5; configurável com `--jobs` |
| Cálculo em lotes de 10 | Compartilha `--jobs`, padrão 5 |
| Node.js/Bun, TypeScript e bibliotecas npm | Rust `std`, sem dependências externas |

## Decisões

O parser trabalha com `OsString`, permitindo caminhos não UTF-8 no Unix, opções antes/depois do caminho e `--`. Argumentos desconhecidos, múltiplos caminhos e ausência de caminho retornam erro.

A descoberta é sequencial; cálculo e remoção usam threads com escopo em lotes limitados. Isso evita criar uma thread para cada arquivo e mantém uma implementação pequena, sem runtime assíncrono. Não foi realizada comparação de desempenho com o original.

O scanner inclui a própria raiz quando seu nome é `node_modules`. Links e reparse points são ignorados. O tamanho usa apenas arquivos regulares e não segue links. Caminhos na lista de candidatos são escapados para evitar interpretação de caracteres de controle pelo terminal.

Erros de leitura não desaparecem: são apresentados como avisos e causam saída 1. Pastas acessíveis continuam disponíveis para limpeza após confirmação. O resultado distingue sucesso e falha e soma apenas o tamanho estimado dos candidatos cuja remoção terminou sem erro; uma falha pode ter removido parte da pasta.

A remoção revalida escopo, nome e identidade antes de usar `remove_dir_all`. As verificações não eliminam corridas com processos que alterem a árvore no intervalo de validação: não executar sobre árvores hostis ou em alteração.

## Validação

A suíte cobre descoberta em múltiplos projetos, dependências aninhadas, conteúdo oculto, raiz direta, caminhos inválidos, limite de workers, dry run com confirmação pulada, EOF/cancelamento, confirmação positiva, exclusão em lote, códigos de saída e candidato desaparecido. No Unix, cobre links externos/quebrados, troca de candidato/ancestral e caminhos não UTF-8.

CI configurada para Linux, macOS e Windows. Testes específicos de symlinks e troca de inode são Unix; a matriz Windows valida o fluxo normal e a compilação do tratamento de reparse points, sem afirmar cobertura específica de junctions.
