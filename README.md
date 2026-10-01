# Rust Node Modules Cleanup

CLI em Rust para encontrar pastas `node_modules`, mostrar seus tamanhos e removê-las em lote com confirmação.

Reimplementação inspirada em [sebastianekstrom/node-modules-cleanup](https://github.com/sebastianekstrom/node-modules-cleanup). Executável nativo, sem Node.js, Bun ou dependências Cargo externas.

## Instalação

Requer Rust e Cargo **1.85 ou superior** para compilar. Depois de compilado, o executável não requer Rust instalado.

```bash
git clone https://github.com/mdnbras/rust-node-modules-cleanup.git
cd rust-node-modules-cleanup
cargo install --path . --locked
```

Se o repositório for privado, autentique seu Git antes de clonar. O pacote não foi publicado no crates.io.

## Uso

```bash
# Inspecionar sem apagar nem solicitar confirmação
rust-node-modules-cleanup ~/projects --dry

# Listar tamanhos e pedir confirmação antes de excluir
rust-node-modules-cleanup ~/projects

# Excluir todas as pastas encontradas sem confirmação
rust-node-modules-cleanup ~/projects --skip-confirmation

# Limitar operações simultâneas
rust-node-modules-cleanup ~/projects --jobs 2 --dry

# Caminhos com espaços
rust-node-modules-cleanup "/home/user/my projects" --dry
```

Windows (PowerShell):

```powershell
rust-node-modules-cleanup.exe "C:\Users\seu-usuario\projects" --dry
```

| Argumento | Comportamento |
| --- | --- |
| `<path>` | Diretório obrigatório, relativo ou absoluto |
| `--dry`, `--dry-run` | Apenas listar; tem precedência sobre `-y` |
| `--skip-confirmation`, `-y` | Excluir sem perguntar |
| `--jobs N` | De 1 a 64 operações simultâneas; padrão 5 |
| `--help`, `-h`, `--h` | Ajuda |
| `--version`, `-V`, `--v` | Versão |
| `--` | Encerrar opções, permitindo caminhos iniciados por hífen |

A busca ignora subdiretórios cujo nome começa com ponto e não entra em uma pasta `node_modules` já identificada. Pastas ocultas e dependências aninhadas **dentro** dela entram no cálculo de tamanho e na exclusão. Também é possível informar uma pasta `node_modules` diretamente.

A confirmação aceita `y`, `yes`, `s` e `sim`, sem diferenciar maiúsculas; mantém ainda a resposta especial do original, `kör bara kör!`. Enter, EOF ou qualquer outra resposta cancela. Exclusão é permanente, sem lixeira; execute `npm install`, `npm ci` ou o gerenciador do projeto para restaurar as dependências.

## Segurança e limites

- Links simbólicos e, no Windows, junctions/reparse points não são percorridos. Links internos são removidos junto com a pasta; os destinos não são contabilizados nem apagados por seguimento de links.
- Antes da exclusão, o caminho é revalidado quanto ao nome, escopo, resolução canônica e identidade do diretório (device/inode no Unix; criação no Windows).
- Pare instalações, builds e processos que alterem a árvore durante a operação. As verificações reduzem riscos, mas não constituem uma sandbox contra alterações concorrentes maliciosas; existe intervalo entre verificação e remoção.
- Erros de leitura geram avisos. Uma busca parcial pode continuar com as pastas encontradas, mas termina com código 1. Falhas de exclusão não impedem as demais tentativas e não entram no contador de pastas removidas.
- O tamanho é a soma dos bytes lógicos dos arquivos regulares, não uma medida exata de espaço físico liberado: hard links, compressão, arquivos esparsos e alterações concorrentes podem mudar o resultado.
- Não há benchmark comparativo: a escolha de Rust não implica uma promessa medida de desempenho.

| Código de saída | Significado |
| --- | --- |
| `0` | Operação completa, dry run, cancelamento ou busca completa sem resultados |
| `1` | Argumentos inválidos, falha de acesso/exclusão ou busca/tamanho incompleto |

## Desenvolvimento

```bash
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
cargo run -- ./algum-diretorio --dry
```

Binário local: `target/release/rust-node-modules-cleanup` (sufixo `.exe` no Windows).

O GitHub Actions executa formatação, Clippy, testes e build em Linux, Windows e macOS, além de verificar o Rust mínimo no Linux. Os executáveis produzidos ficam nos artifacts do workflow; não são releases publicados.

## Estrutura

- `src/cli.rs`: argumentos e confirmação.
- `src/scan.rs`: descoberta, tamanho, validação e exclusão.
- `src/main.rs`: interface de terminal, execução concorrente e resumo.
- `tests/cleanup.rs`: testes com árvores temporárias e invocação do executável real.
- [docs/ANALISE.md](docs/ANALISE.md): análise do projeto original e decisões da migração.

## Licença e créditos

MIT. Projeto original de Sebastian Ekström; sua licença está preservada em [LICENSE-UPSTREAM](LICENSE-UPSTREAM). Esta versão mantém o objetivo e as opções principais, com diferenças documentadas, sem reproduzir a apresentação visual do CLI original.
