"""Compare unchanged JS core with Rust core on disposable, identical fixtures."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import shutil
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
PROFILES = {
    "small": {"projects": 5, "packages": 8, "files": 16, "levels": 2, "file_bytes": 1024},
    "medium": {"projects": 25, "packages": 12, "files": 24, "levels": 3, "file_bytes": 1024},
    "large": {"projects": 50, "packages": 12, "files": 24, "levels": 3, "file_bytes": 1024},
}
METRICS = ("scan_ms", "delete_ms", "core_ms", "process_ms")


def fixture(root, config):
    """New real files every invocation: no hardlinks, copies, or shared data trees."""
    payload = b"x" * config["file_bytes"]
    for project in range(config["projects"]):
        base = root / f"project-{project:04}" / "node_modules"
        for level in range(config["levels"]):
            for package in range(config["packages"]):
                dest = base / f"package-{package:03}"
                dest.mkdir(parents=True, exist_ok=True)
                for file in range(config["files"]):
                    (dest / f"file-{file:03}.js").write_bytes(payload)
            base = base / "nested-dependency" / "node_modules"
    # Validate that both versions retain files outside their cleanup scope.
    sentinels = [root / "keep.txt", root / ".hidden" / "node_modules" / "keep.txt"]
    sentinels += [root / f"project-{n:04}" / "package.json" for n in range(config["projects"])]
    for file in sentinels:
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_bytes(b"preserve")
    return sentinels


def summary(values):
    ordered = sorted(values)
    return {
        "median": statistics.median(values),
        "mean": statistics.mean(values),
        "stdev": statistics.stdev(values) if len(values) > 1 else 0,
        "min": ordered[0],
        "max": ordered[-1],
    }


def command_output(args):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT).strip()


def storage_info(path):
    if platform.system() == "Linux" and shutil.which("df"):
        return command_output(["df", "-T", str(path)])
    return "Filesystem type not collected on this platform"


def source_hash():
    digest = hashlib.sha256()
    for path in sorted([*ROOT.glob("src/*.rs"), ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "examples/benchmark.rs"]):
        digest.update(str(path.relative_to(ROOT)).encode())
        digest.update(path.read_bytes())
    return digest.hexdigest()


def report_markdown(report):
    lines = ["# Benchmark Rust × JavaScript", "", f"UTC: {report['created_at']}", "",
             f"Ambiente: `{report['environment']['platform']}`; CPU: {report['environment']['cpu']}; CPUs lógicas visíveis: {report['environment']['cpus']}.",
             f"Node: `{report['environment']['node']}`; Rust: `{report['environment']['rustc']}`.",
             f"Upstream JS: `{report['upstream']['commit']}`; fonte Rust: `{report['rust_source_sha256']}`.", "",
             "Tempos em milissegundos. Razão = mediana JS / mediana Rust; > 1 favorece Rust, < 1 favorece JS.", "",
             "| Cenário | Etapa | Rust mediana | JS mediana | Razão JS/Rust | Rust desvio padrão | JS desvio padrão |",
             "| --- | --- | ---: | ---: | ---: | ---: | ---: |"]
    labels = {"scan_ms": "Busca + tamanho", "delete_ms": "Exclusão", "core_ms": "Núcleo total", "process_ms": "Processo completo (adaptador)"}
    for name, result in report["profiles"].items():
        for metric in METRICS:
            rust = result["summary"]["rust"][metric]
            js = result["summary"]["javascript"][metric]
            lines.append(f"| {name} | {labels[metric]} | {rust['median']:.2f} | {js['median']:.2f} | {js['median']/rust['median']:.2f}× | {rust['stdev']:.2f} | {js['stdev']:.2f} |")
    lines += ["", "## Armazenamento", "", "```text", report['environment'].get('storage', 'Not collected'), "```", "", "## Método e limites", "",
              f"- {report['settings']['runs']} amostras medidas e {report['settings']['warmups']} aquecimentos por implementação e cenário. Ordem alternada, início sorteado com seed {report['settings']['seed']}.",
              "- Fixture reconstruída fora da medição antes de cada execução. Cache do SO não é limpo: este é um cenário recém-criado/cache aquecido, não cold-cache.",
              "- As etapas são medidas dentro dos adaptadores; processo completo inclui inicialização do runtime e adaptador, não a interface CLI original.",
              "- JS usa as funções originais sem alterar seus algoritmos; progresso é silenciado. Rust usa a biblioteca e o mesmo lote de exclusão do CLI. Sem prompts nem atrasos artificiais de dry run.",
              "- Concorrência padrão preservada: Rust 5 para tamanho/exclusão; JS 10 para tamanho e 5 para exclusão, com paralelismo interno próprio. Não é um teste isolado da linguagem.",
              "- Instalação, compilação, geração das fixtures e validação dos arquivos ficam fora da medição. Cada processo precisa terminar com sucesso, informar contagem/tamanho corretos e remover apenas os alvos.",
              "- Ambiente compartilhado e armazenamento influenciam os resultados. Não extrapolar estas razões para outros computadores ou workloads. Memória e consumo de CPU não são medidos.", "",
              "## Fixtures", "", "| Perfil | Projetos | Pacotes/nível | Arquivos/pacote | Níveis | Bytes/arquivo | Arquivos alvo |",
              "| --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for name, result in report["profiles"].items():
        c = result["fixture"]
        lines.append(f"| {name} | {c['projects']} | {c['packages']} | {c['files']} | {c['levels']} | {c['file_bytes']} | {result['file_count']} |")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profiles", nargs="+", choices=PROFILES, default=["small", "medium"])
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--timeout", type=float, default=300)
    parser.add_argument("--work-dir", type=Path, help="Existing directory on the filesystem to measure; only a new temporary child is used")
    parser.add_argument("--output", type=Path, default=ROOT / "benchmarks/results/latest.json")
    args = parser.parse_args()
    if args.runs < 2 or args.warmups < 0 or args.timeout <= 0:
        parser.error("Require runs >= 2, warmups >= 0 and timeout > 0")
    rust = ROOT / "target/release/examples" / ("benchmark.exe" if os.name == "nt" else "benchmark")
    js = ROOT / "benchmarks/.cache/js-benchmark.mjs"
    if not rust.is_file() or not js.is_file():
        parser.error("Build adapters first; see benchmarks/README.md")
    node = shutil.which("node")
    if not node:
        parser.error("Node.js is required")
    commands = {"rust": [str(rust)], "javascript": [node, str(js)]}
    cpu = platform.processor() or "not reported"
    if Path("/proc/cpuinfo").exists():
        cpu = next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines() if line.startswith("model name")), cpu)
    report = {
        "created_at": datetime.now(timezone.utc).isoformat(),
        "environment": {"platform": platform.platform(), "cpu": cpu, "cpus": os.cpu_count(),
                        "storage": storage_info(args.work_dir or Path(tempfile.gettempdir())),
                        "node": command_output([node, "--version"]), "rustc": command_output(["rustc", "--version"]),
                        "python": platform.python_version(), "uv_threadpool_size": os.environ.get("UV_THREADPOOL_SIZE", "default (4)")},
        "upstream": json.loads((ROOT / "benchmarks/.cache/upstream.json").read_text()),
        "rust_source_sha256": source_hash(),
        "adapter_sha256": {name: hashlib.sha256(Path(command[-1]).read_bytes()).hexdigest() for name, command in commands.items()},
        "settings": {"runs": args.runs, "warmups": args.warmups, "seed": args.seed, "timeout": args.timeout,
                     "work_dir": str((args.work_dir or Path(tempfile.gettempdir())).resolve())},
        "profiles": {},
    }
    rng = random.Random(args.seed)
    for profile in args.profiles:
        config = PROFILES[profile]
        count = config["projects"] * config["packages"] * config["files"] * config["levels"]
        samples = {name: [] for name in commands}
        warmups = {name: [] for name in commands}
        first = rng.choice(list(commands))
        second = next(name for name in commands if name != first)
        for iteration in range(args.warmups + args.runs):
            order = [first, second] if iteration % 2 == 0 else [second, first]
            for position, name in enumerate(order):
                with tempfile.TemporaryDirectory(prefix="nmc-benchmark-", dir=args.work_dir) as temporary:
                    root = Path(temporary)
                    sentinels = fixture(root, config)
                    started = time.perf_counter_ns()
                    completed = subprocess.run(commands[name] + [str(root)], capture_output=True, text=True, timeout=args.timeout)
                    elapsed = (time.perf_counter_ns() - started) / 1_000_000
                    if completed.returncode:
                        raise RuntimeError(f"{name} failed: {completed.stderr}\n{completed.stdout}")
                    sample = json.loads(completed.stdout)
                    if sample["folders"] != config["projects"] or sample["bytes"] != count * config["file_bytes"]:
                        raise RuntimeError(f"{name}: incorrect fixture counts: {sample}")
                    if any((root / f"project-{n:04}" / "node_modules").exists() for n in range(config["projects"])):
                        raise RuntimeError(f"{name}: some target directories remain")
                    if any(not path.is_file() or path.read_bytes() != b"preserve" for path in sentinels):
                        raise RuntimeError(f"{name}: file outside cleanup scope was modified")
                    sample.update(process_ms=elapsed, iteration=iteration, position=position)
                    (warmups if iteration < args.warmups else samples)[name].append(sample)
                    print(f"{profile} {iteration + 1}/{args.warmups + args.runs} {name}: {elapsed:.2f} ms", flush=True)
        report["profiles"][profile] = {"fixture": config, "file_count": count, "samples": samples, "warmups": warmups,
            "summary": {name: {metric: summary([s[metric] for s in rows]) for metric in METRICS} for name, rows in samples.items()}}
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf8")
        args.output.with_suffix(".md").write_text(report_markdown(report), encoding="utf8")
    print(f"Reports: {args.output} and {args.output.with_suffix('.md')}")


if __name__ == "__main__":
    main()
