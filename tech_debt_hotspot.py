from __future__ import annotations

import csv
import math
import sys
from collections import Counter
from dataclasses import dataclass
from enum import Enum, unique
from pathlib import Path
from typing import Final, Iterable, Iterator, Mapping

import click
import radon.metrics
import sh
from loguru import logger
from tqdm import tqdm

ROOT_PATH: Final = Path(".")
MINIMUM_MAINTAINABILITY_INDEX: Final = 0.01


@unique
class PathType(Enum):
    PACKAGE = "package"
    MODULE = "module"


@dataclass
class PathMetrics:
    path: Path
    path_type: PathType
    maintainability_index: float = math.inf  # percentage from 0 to 100
    changes_count: int = 0

    @property
    def hotspot_index(self) -> float:
        return self.changes_count / (self.maintainability_index / 100)


@dataclass(frozen=True)
class FileChanges:
    filename: Path
    changes_count: int


@dataclass(frozen=True)
class FileMaintainability:
    path: Path
    maitainability_index: float  # percentage from 0 to 100


def maintainability_index_iter(directory: Path, /) -> Iterator[FileMaintainability]:
    logger.info("Collecting maintainability indexes ...")

    filenames = list(directory.glob("**/*.py"))

    for filename in tqdm(filenames, unit="file", desc="Processing files"):
        code = filename.read_text()
        maintainability_index = radon.metrics.mi_visit(code, multi=True)

        # We cannot have a 0% maintainability index so we set a very low number
        maintainability_index = max(MINIMUM_MAINTAINABILITY_INDEX, maintainability_index)

        yield FileMaintainability(
            path=filename.relative_to(directory), maitainability_index=maintainability_index
        )


def changes_count_iter(directory: Path, /) -> Iterator[FileChanges]:
    logger.info("Collecting changes count ...")

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

    logger.info("Counting changes ...")

    changes_counter = Counter(filenames)
    changes_count = (
        FileChanges(filename=filename, changes_count=count)
        for filename, count in changes_counter.items()
    )

    yield from changes_count


def filename_parent_iter(filename: Path, /) -> Iterator[Path]:
    yield from filename.parents
    yield filename


def get_path_type(filename: Path, /) -> PathType:
    return PathType.MODULE if filename.suffix == ".py" else PathType.PACKAGE


def update_maitainability_metrics(
    metrics: dict[Path, PathMetrics], maitainability_data: Iterable[FileMaintainability], /
) -> None:
    logger.info("Updating maintainability metrics ...")

    for maitainability in maitainability_data:
        for parent in filename_parent_iter(maitainability.path):
            path_metric = metrics.setdefault(
                parent, PathMetrics(path=parent, path_type=get_path_type(parent))
            )
            path_metric.maintainability_index = min(
                path_metric.maintainability_index, maitainability.maitainability_index
            )


def update_changes_count_metrics(
    metrics: dict[Path, PathMetrics], changes_count_data: Iterable[FileChanges], /
) -> None:
    logger.info("Updating changes count metrics ...")

    for changes_count in changes_count_data:
        for parent in filename_parent_iter(changes_count.filename):
            path_metrics = metrics.setdefault(
                parent, PathMetrics(path=parent, path_type=get_path_type(parent))
            )
            path_metrics.changes_count += changes_count.changes_count


def print_metrics(metrics: Mapping[Path, PathMetrics], /) -> None:
    logger.info("Printing metrics to stdout ...")

    fieldnames = ["path", "path_type", "maintainability_index", "changes_count", "hotspot_index"]

    writer = csv.DictWriter(sys.stdout, fieldnames=fieldnames)
    writer.writeheader()

    for metric in metrics.values():
        writer.writerow(
            {
                "path": metric.path,
                "path_type": metric.path_type.value,
                "maintainability_index": metric.maintainability_index,
                "changes_count": metric.changes_count,
                "hotspot_index": metric.hotspot_index,
            }
        )


@click.command(help="Collect tech debt hotspot stats for the given directory")
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
def main(directory: Path) -> None:
    maitainability_data = maintainability_index_iter(directory)
    changes_count_data = changes_count_iter(directory)

    metrics = {ROOT_PATH: PathMetrics(path=ROOT_PATH, path_type=PathType.PACKAGE)}

    update_maitainability_metrics(metrics, maitainability_data)
    update_changes_count_metrics(metrics, changes_count_data)

    print_metrics(metrics)


if __name__ == "__main__":
    main()
