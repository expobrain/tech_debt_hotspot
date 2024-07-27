import json
from collections import Counter
from pathlib import Path
from typing import Iterator

import click
import radon.metrics
import sh


def maintainability_index_iter(directory: Path, /) -> Iterator[tuple[Path, dict[str, float]]]:
    for filename in directory.glob("**/*.py"):
        code = filename.read_text()
        mi_index = radon.metrics.mi_visit(code, multi=True)

        yield filename.relative_to(directory), mi_index


def changes_count_iter(directory: Path, /) -> Iterator[tuple[Path, int]]:
    git_log = sh.git(
        "log",
        "--name-only",
        "--relative",
        "--pretty=format:",
        directory,
        _cwd=directory,
        _tty_out=False,
    )

    filenames_str: filter[str] = filter(None, git_log.split("\n"))
    filenames = (directory / filename_str for filename_str in filenames_str)
    filenames = (filename for filename in filenames if filename.suffix == ".py")
    filenames = (filename.resolve().relative_to(directory) for filename in filenames)

    changes_count = Counter(filenames)

    yield from changes_count.items()


@click.group()
def main() -> None:
    pass


@main.command(help="Collect maitainability stats for the given directory")
@click.option("-j", "--json", "json_output", is_flag=True, help="Output in JSON format")
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
def mi(directory: Path, json_output: bool) -> None:
    maintainability_index = maintainability_index_iter(directory)

    if json_output:
        data = {filename.as_posix(): mi_index for filename, mi_index in maintainability_index}
        click.echo(json.dumps(data))
    else:
        for filename, mi_index in maintainability_index:
            click.echo(f"{filename} --> {mi_index}")


@main.command(help="Collect number of changes per file per given period")
@click.option("-j", "--json", "json_output", is_flag=True, help="Output in JSON format")
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
def changes(directory: Path, json_output: bool) -> None:
    changes_count = changes_count_iter(directory)

    if json_output:
        data = {filename.as_posix(): changes_count for filename, changes_count in changes_count}
        click.echo(json.dumps(data))
    else:
        for filename, count in changes_count:
            click.echo(f"{filename} --> {count}")


if __name__ == "__main__":
    main()
