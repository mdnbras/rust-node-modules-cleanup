"""Render the recorded benchmark: python benchmarks/plot.py (requires matplotlib)."""
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
from matplotlib.ticker import MultipleLocator

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "docs/benchmarks/2026-10-01-linux.json"
OUTPUT = ROOT / "docs/assets/benchmark-rust-vs-javascript.png"


def number(value):
    return f"{value:.2f}".replace(".", ",")


def main():
    report = json.loads(SOURCE.read_text())
    rust, javascript = "#D95431", "#2867BD"
    ink, muted, background = "#14263D", "#52637A", "#F5F7FB"
    plt.rcParams.update({"font.family": "DejaVu Sans", "font.size": 12})
    fig, axes = plt.subplots(1, 2, figsize=(16, 9.8))
    fig.patch.set_facecolor(background)
    fig.subplots_adjust(left=0.16, right=0.95, bottom=0.24, top=0.68, wspace=0.62)
    fig.text(0.055, 0.94, "RUST × JAVASCRIPT", fontsize=13, weight="bold", color=muted)
    fig.text(0.055, 0.884, "Benchmark de limpeza de node_modules", fontsize=27, weight="bold", color=ink)
    fig.text(0.055, 0.84, "Mediana de 7 execuções  •  Tempo em milissegundos  •  Menor é melhor", fontsize=13, color=muted)
    fig.legend(handles=[Patch(color=rust, label="Rust"), Patch(color=javascript, label="JavaScript / Node.js")],
               loc="upper left", bbox_to_anchor=(0.05, 0.808), frameon=False, ncol=2, fontsize=13)
    metrics = ["scan_ms", "delete_ms", "core_ms", "process_ms"]
    labels = ["Busca +\ntamanho", "Exclusão", "Núcleo\ntotal", "Processo completo\n(adaptador)"]
    for ax, name, title, tick in zip(axes, ["small", "medium"],
                                    ["1.280 arquivos · 5 projetos", "21.600 arquivos · 25 projetos"], [40, 200]):
        data = report["profiles"][name]["summary"]
        ax.set_facecolor(background)
        ax.set_title(title, loc="left", fontsize=14, weight="bold", color=ink, pad=20)
        values = {impl: [data[impl][metric]["median"] for metric in metrics] for impl in ["rust", "javascript"]}
        limit = max(values["javascript"] + values["rust"]) * 1.31
        for impl, color, offset in [("rust", rust, -0.18), ("javascript", javascript, 0.18)]:
            bars = ax.barh([i + offset for i in range(4)], values[impl], height=0.28, color=color, zorder=3)
            for bar, value in zip(bars, values[impl]):
                ax.text(value + limit * 0.022, bar.get_y() + bar.get_height() / 2,
                        number(value), va="center", fontsize=11, color=ink, weight="bold")
        ax.set_yticks(range(4), labels, color=ink, fontsize=12)
        ax.invert_yaxis()
        ax.set_xlim(0, limit)
        ax.xaxis.set_major_locator(MultipleLocator(tick))
        ax.set_xlabel("Tempo (ms) · escala linear a partir de zero", color=muted, fontsize=10, labelpad=12)
        ax.grid(axis="x", color="#DDE3EC", zorder=0)
        ax.tick_params(axis="both", length=0, labelcolor=muted, pad=9)
        for spine in ax.spines.values():
            spine.set_visible(False)
    ratio = report["profiles"]["medium"]["summary"]["javascript"]["core_ms"]["median"] / report["profiles"]["medium"]["summary"]["rust"]["core_ms"]["median"]
    fig.text(0.055, 0.145, f"{number(ratio)}×", fontsize=29, weight="bold", color=rust)
    fig.text(0.165, 0.151, "razão JS/Rust no núcleo total do cenário de 21.600 arquivos", fontsize=13, color=ink)
    fig.text(0.055, 0.098, "Linux x64 · Node.js 24.19.0 · Rust 1.99.0 · 01/10/2026 · Eixos com escalas independentes", fontsize=10, color=muted)
    fig.text(0.055, 0.065, "Carga sintética em filesystem overlay, cache aquecido; 1 aquecimento por implementação e cenário.", fontsize=10, color=muted)
    fig.text(0.055, 0.036, "Funções reais, sem interface CLI nem atrasos do dry run. Resultados específicos deste ambiente; não são garantia universal.", fontsize=10, color=muted)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(OUTPUT, dpi=150, facecolor=background)
    plt.close(fig)
    print(OUTPUT)


if __name__ == "__main__":
    main()
