from pathlib import Path
from typing import Iterator

import click
import radon.metrics


def maintainability_index_iter(directory: Path) -> Iterator[tuple[Path, dict[str, float]]]:
    for filename in directory.glob("**/*.py"):
        code = filename.read_text()
        mi_index = radon.metrics.mi_visit(code, multi=True)

        yield filename.relative_to(directory), mi_index


@click.group()
def main() -> None:
    pass


@main.command(help="Collect maitainability stats for the given directory")
@click.argument(
    "directory",
    type=click.Path(
        exists=True,
        file_okay=False,
        dir_okay=True,
        readable=True,
        resolve_path=True,
        path_type=Path,
    ),
)
def mi(directory: Path) -> None:
    for filename, mi_index in maintainability_index_iter(directory):
        click.echo(f"{filename} --> {mi_index}")


if __name__ == "__main__":
    main()
