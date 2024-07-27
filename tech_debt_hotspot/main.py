from __future__ import annotations

import csv
import json
import math
import sys
from collections import Counter
from enum import Enum, unique
from pathlib import Path
from typing import Iterator, Sequence

import click
import radon.metrics
import sh
from pydantic import BaseModel

ROOT_PATH = Path(".")


@unique
class PathType(Enum):
    PACKAGE = "package"
    MODULE = "module"


class PathMetrics(BaseModel):
    path: Path
    path_type: PathType
    maintainability_index: float = math.inf  # percentage from 0 to 100
    changes_count: int = 0

    @property
    def hotspot_index(self) -> float:
        return self.changes_count / (self.maintainability_index / 100)


class FileChanges(BaseModel):
    filename: Path
    changes_count: int


class FileMaintainability(BaseModel):
    filename: Path
    maitainability_index: float  # percentage from 0 to 100


def load_maitanability_data(filename: Path, /) -> list[FileMaintainability]:
    data_raw = json.load(filename.open())
    data = [
        FileMaintainability(filename=key, maitainability_index=value)
        for key, value in data_raw.items()
    ]

    return data


def load_changes_count_data(filename: Path, /) -> list[FileChanges]:
    data_raw = json.load(filename.open())
    data = [FileChanges(filename=key, changes_count=value) for key, value in data_raw.items()]

    return data


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


def filename_parent_iter(filename: Path, /) -> Iterator[Path]:
    yield from filename.parents
    yield filename


def get_path_type(filename: Path, /) -> bool:
    return PathType.MODULE if filename.suffix == ".py" else PathType.PACKAGE


def update_maitainability_metrics(
    metrics: dict[Path, PathMetrics], maitainability_data: Sequence[FileMaintainability], /
) -> None:
    for maitainability in maitainability_data:
        for parent in filename_parent_iter(maitainability.filename):
            path_metric = metrics.setdefault(
                parent, PathMetrics(path=parent, path_type=get_path_type(parent))
            )
            path_metric.maintainability_index = min(
                path_metric.maintainability_index, maitainability.maitainability_index
            )


def update_changes_count_metrics(
    metrics: dict[Path, PathMetrics], changes_count_data: Sequence[FileChanges], /
) -> None:
    for changes_count in changes_count_data:
        for parent in filename_parent_iter(changes_count.filename):
            path_metrics = metrics.setdefault(
                parent, PathMetrics(path=parent, path_type=get_path_type(parent))
            )
            path_metrics.changes_count += changes_count.changes_count


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


@main.command(help="Combine maintainability index and changes count for visualisation")
@click.option(
    "-m",
    "--maintainability",
    "maitainability_filename",
    type=click.Path(
        exists=True,
        file_okay=True,
        dir_okay=False,
        readable=True,
        resolve_path=True,
        path_type=Path,
    ),
)
@click.option(
    "-c",
    "--changes",
    "changes_filename",
    type=click.Path(
        exists=True,
        file_okay=True,
        dir_okay=False,
        readable=True,
        resolve_path=True,
        path_type=Path,
    ),
)
def combine(maitainability_filename: Path, changes_filename: Path) -> None:
    maitainability_data = load_maitanability_data(maitainability_filename)
    changes_count_data = load_changes_count_data(changes_filename)

    metrics = {ROOT_PATH: PathMetrics(path=ROOT_PATH, path_type=PathType.PACKAGE)}

    update_maitainability_metrics(metrics, maitainability_data)
    update_changes_count_metrics(metrics, changes_count_data)

    # Print to stdout
    fieldnames = ["path", "path_type", "maintainability_index", "changes_count", "hotspot_index"]

    writer = csv.DictWriter(sys.stdout, fieldnames=fieldnames)
    writer.writeheader()

    for metric in sorted(metrics.values(), key=lambda metric: metric.path):
        writer.writerow(
            {
                "path": metric.path,
                "path_type": metric.path_type.value,
                "maintainability_index": metric.maintainability_index,
                "changes_count": metric.changes_count,
                "hotspot_index": metric.hotspot_index,
            }
        )


if __name__ == "__main__":
    main()
