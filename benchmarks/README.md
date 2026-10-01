# Benchmark Rust × JavaScript

Compara busca + cálculo de tamanho, exclusão, núcleo total e tempo do processo com o projeto original. O benchmark não pressupõe que Rust vencerá.

## Executar localmente

Requisitos: Rust/Cargo 1.85+, Node.js 22+ (use 24 para reproduzir o ambiente medido), npm, Git e Python 3.10+. Os comandos abaixo partem da raiz do repositório.

```bash
git clone https://github.com/sebastianekstrom/node-modules-cleanup.git benchmarks/.cache/upstream
git -C benchmarks/.cache/upstream checkout 74e099ee05a57c683d9e7a1887b806f9f7a08d86
npm ci --prefix benchmarks --ignore-scripts --no-audit --no-fund
node benchmarks/build.mjs
cargo build --release --locked --example benchmark
python benchmarks/run.py --profiles small medium --runs 7 --warmups 1
```

Se já tiver um checkout limpo do original nesse commit, use `node benchmarks/build.mjs /caminho/do/original` em vez de clonar novamente. Os arquivos originais não são modificados. Esbuild compila um adaptador que importa as funções TypeScript diretamente do checkout original; as dependências auxiliares têm versões fixadas em `package-lock.json`.

Saídas: `benchmarks/results/latest.json` (todas as amostras, inclusive aquecimentos, metadados e estatísticas) e `latest.md` (relatório legível). Altere com `--output caminho/resultado.json`.

Para medir outro disco, use `--work-dir /caminho/existente/no/disco`. O script cria uma subpasta temporária própria e recria todos os arquivos antes de cada medição; nunca fornece a pasta indicada diretamente aos removedores. Não execute os adaptadores isolados sobre seus projetos: eles apagam `node_modules` sem confirmação.

## GitHub Actions

Abra **Actions → Benchmark Rust vs JavaScript → Run workflow**. Selecione `small`, `medium`, `large` ou `all`, número de amostras e aquecimentos. Baixe os relatórios pelo artifact `benchmark-results` e veja a tabela no resumo da execução. O workflow é manual e não falha por um limiar arbitrário de desempenho; falhas de execução ou de integridade dos dados são erros.

Os runners compartilhados do GitHub servem para comparação exploratória, não para afirmar ganhos universais. Compare resultados obtidos na mesma máquina, runtime e sistema de arquivos.

## Fixtures determinísticas

| Perfil | Projetos | Pacotes por nível | Arquivos por pacote | Níveis | Arquivos alvo | Bytes/arquivo |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small | 5 | 8 | 16 | 2 | 1.280 | 1.024 |
| medium | 25 | 12 | 24 | 3 | 21.600 | 1.024 |
| large | 50 | 12 | 24 | 3 | 43.200 | 1.024 |

Cada projeto tem uma cadeia de dependências aninhadas. Cada nível contém a quantidade indicada de pacotes; os números não representam uma árvore exponencial. Também há arquivos sentinela fora do alvo e dentro de um projeto oculto. Os conteúdos e nomes são iguais nas duas implementações. Não há links simbólicos, hardlinks ou erros de permissão na carga comparada.

## O que é medido

- `scan_ms`: descobrir pastas e calcular seus tamanhos.
- `delete_ms`: remover as pastas já identificadas.
- `core_ms`: busca, cálculo e remoção no processo; soma das duas etapas.
- `process_ms`: subprocesso completo do adaptador, incluindo inicialização/encerramento do runtime e serialização JSON.

O JS executa `findNodeModulesFolders`, `calculateSizeOfNodeModulesDirs` e `deleteFolders` do commit fixado. Rust executa `scan::scan` e `Scan::delete`, com os mesmos lotes de remoção do CLI. O adaptador JS silencia o progresso; Rust não renderiza progresso no adaptador. Não se mede a interface completa de nenhum dos CLIs.

**Não usamos `--dry` do original**: esse modo inclui atrasos aleatórios simulando exclusão, o que tornaria a comparação enganosa. Mantemos a concorrência padrão das implementações: Rust com 5 workers para tamanho/exclusão; JS em lotes de 10 para tamanho e 5 para exclusão, com concorrência interna das funções originais. Portanto, o resultado compara implementações e suas escolhas de I/O, não apenas linguagens.

O fixture é criado fora do cronômetro, imediatamente antes de cada execução. Não se limpa o cache do SO e não se força sincronização física de disco: é uma carga recém-criada/cache aquecido. A ordem alterna entre implementações, com início escolhido por seed fixa. Cada amostra usa um processo novo; os aquecimentos exercitam runtime e sistema de arquivos, mas não preservam JIT entre processos.

O script exige saída zero, contagem e tamanho exatos, desaparecimento de todos os alvos e integridade dos arquivos sentinela. Reporta mediana, média, desvio padrão amostral, mínimo, máximo e amostras brutas. A razão JS/Rust usa medianas: maior que 1 favorece Rust, menor que 1 favorece JS. Não são medidos memória, CPU nem significância estatística. Compilação, instalação, geração e validação ficam fora da medição.

## Resultados registrados

Consulte `docs/benchmarks/` para medições identificadas por data e ambiente. Os hashes do código Rust e dos adaptadores compilados permitem distinguir resultados de builds diferentes.
