from __future__ import annotations

import csv
import math
import sys
from collections import Counter
from collections.abc import Iterable, Iterator, Sequence
from dataclasses import dataclass
from datetime import date, datetime
from enum import Enum, unique
from pathlib import Path
from typing import Final

import click
import radon.metrics
import sh
from loguru import logger
from prettytable import PrettyTable
from tqdm import tqdm

ROOT_PATH: Final = Path(".")
MINIMUM_MAINTAINABILITY_INDEX: Final = 0.01


@unique
class OutputType(Enum):
    CSV = "csv"
    MARKDOWN = "markdown"


@unique
class PathType(Enum):
    PACKAGE = "package"
    MODULE = "module"


@dataclass
class PathMetrics:
    path: Path
    path_type: PathType
    halsteads_volume: float = 0
    cyclomatic_complexity: int = 0
    loc: int = 0
    comments_percentage: float = 0
    maintainability_index: float = math.inf  # percentage from 0 to 100
    changes_count: int = 0

    @property
    def hotspot_index(self) -> float:
        return self.changes_count / (self.maintainability_index / 100)

    def is_deleted(self) -> bool:
        return self.maintainability_index == math.inf


FIELDNAMES: Final = list(PathMetrics.__annotations__.keys()) + ["hotspot_index"]


@dataclass(frozen=True)
class FileChanges:
    filename: Path
    changes_count: int


def is_excluded(path: Path, excluded: set[Path], /) -> bool:
    for excluded_path in excluded:
        try:
            path.relative_to(excluded_path)
            return True
        except ValueError:
            continue

    return False


def maintainability_index_iter(directory: Path, exclude: set[Path], /) -> Iterator[PathMetrics]:
    logger.info("Collecting maintainability indexes ...")

    filenames = [path for path in directory.rglob("*.py") if not is_excluded(path, exclude)]

    for filename in tqdm(filenames, unit="file", desc="Processing files"):
        code = filename.read_text()
        halsteads_volume, cyclomatic_complexity, loc, comments_percentage = (
            radon.metrics.mi_parameters(code, count_multi=True)
        )
        maintainability_index = radon.metrics.mi_compute(
            halsteads_volume, cyclomatic_complexity, loc, comments_percentage
        )

        # We cannot have a 0% maintainability index so we set a very low number
        maintainability_index = max(MINIMUM_MAINTAINABILITY_INDEX, maintainability_index)

        yield PathMetrics(
            path=filename.relative_to(directory),
            path_type=PathType.MODULE,
            halsteads_volume=halsteads_volume,
            cyclomatic_complexity=cyclomatic_complexity,
            loc=loc,
            comments_percentage=comments_percentage,
            maintainability_index=maintainability_index,
        )


def changes_count_iter(
    directory: Path, exclude: set[Path], /, *, since: date | None = None
) -> Iterator[FileChanges]:
    logger.info("Collecting changes count ...")

    command = [
        "log",
        "--name-only",
        "--relative",
        "--pretty=format:",
    ]

    if since is not None:
        command.extend(["--since", since.isoformat()])

    command.append(directory.as_posix())

    git_log = sh.git(*command, _cwd=directory, _tty_out=False)

    filenames_str: filter[str] = filter(None, git_log.split("\n"))
    filenames = (directory / filename_str for filename_str in filenames_str)
    filenames = (filename for filename in filenames if filename.suffix == ".py")
    filenames = (filename.resolve().relative_to(directory) for filename in filenames)
    filenames = (filename for filename in filenames if not is_excluded(filename, exclude))

    logger.info("Counting changes ...")

    changes_counter = Counter(filenames)
    changes_count = (
        FileChanges(filename=filename, changes_count=count)
        for filename, count in changes_counter.items()
    )

    yield from changes_count


def filename_parent_iter(filename: Path, /) -> Iterator[Path]:
    yield filename
    yield from filename.parents


def get_path_type(filename: Path, /) -> PathType:
    return PathType.MODULE if filename.suffix == ".py" else PathType.PACKAGE


def update_maitainability_metrics(
    metrics: dict[Path, PathMetrics], path_metrics_iter: Iterable[PathMetrics], /
) -> None:
    logger.info("Updating maintainability metrics ...")

    for path_metrics in path_metrics_iter:
        for parent in filename_parent_iter(path_metrics.path):
            aggregated_metrics = metrics.setdefault(
                parent, PathMetrics(path=parent, path_type=get_path_type(parent))
            )

            aggregated_metrics.halsteads_volume = max(
                aggregated_metrics.halsteads_volume, path_metrics.halsteads_volume
            )
            aggregated_metrics.cyclomatic_complexity = max(
                aggregated_metrics.cyclomatic_complexity, path_metrics.cyclomatic_complexity
            )
            aggregated_metrics.loc += path_metrics.loc
            # test me
            aggregated_metrics.comments_percentage = (
                (
                    aggregated_metrics.comments_percentage * aggregated_metrics.loc
                    + path_metrics.comments_percentage * path_metrics.loc
                )
                / (aggregated_metrics.loc + path_metrics.loc)
                if (aggregated_metrics.loc + path_metrics.loc) > 0
                else 0
            )
            aggregated_metrics.maintainability_index = min(
                aggregated_metrics.maintainability_index, path_metrics.maintainability_index
            )
            aggregated_metrics.changes_count += path_metrics.changes_count


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


def print_metrics_csv(metrics: Iterable[PathMetrics], /) -> None:
    logger.info("Rendering metrics to csv ...")

    writer = csv.DictWriter(sys.stdout, fieldnames=FIELDNAMES)
    writer.writeheader()

    for metric in metrics:
        writer.writerow(
            {
                "path": metric.path,
                "path_type": metric.path_type.value,
                "halsteads_volume": metric.halsteads_volume,
                "cyclomatic_complexity": metric.cyclomatic_complexity,
                "loc": metric.loc,
                "comments_percentage": metric.comments_percentage,
                "maintainability_index": metric.maintainability_index,
                "changes_count": metric.changes_count,
                "hotspot_index": metric.hotspot_index,
            }
        )


def print_metrics_markdown(metrics: Iterable[PathMetrics], sort_by_field: str, /) -> None:
    logger.info("Rendering metrics to Markdown ...")

    table = PrettyTable()
    table.field_names = FIELDNAMES
    table.align = "r"
    table.align["path"] = "l"  # type: ignore[index]
    table.sortby = sort_by_field
    table.reversesort = sort_by_field != "path"

    for metric in metrics:
        table.add_row(
            [
                metric.path,
                metric.path_type.value,
                metric.halsteads_volume,
                metric.cyclomatic_complexity,
                metric.loc,
                metric.comments_percentage,
                metric.maintainability_index,
                metric.changes_count,
                metric.hotspot_index,
            ]
        )

    sys.stdout.write(str(table))
    sys.stdout.write("\n")


def parse_since(since: str | None) -> date | None:
    if since is None:
        return None

    try:
        return datetime.strptime(since, "%Y-%m-%d").date()
    except ValueError as exc:
        raise click.BadParameter("Invalid date format. Use 'YYYY-MM-DD'") from exc


def get_metrics_iter(metrics: Iterable[PathMetrics], deleted: bool, /) -> Iterable[PathMetrics]:
    if not deleted:
        metrics = (metric for metric in metrics if not metric.is_deleted())

    yield from metrics


@click.command(help="Collect tech debt hotspot stats for the given directory")
@click.option(
    "--exclude",
    "-e",
    multiple=True,
    type=click.Path(
        exists=True,
        file_okay=True,
        dir_okay=True,
        readable=True,
        resolve_path=True,
        path_type=Path,
    ),
    help="Exclude directories from the analysis",
)
@click.option(
    "--since",
    "-s",
    type=str,
    help="Analyze changes since the given date. Date's format is 'YYYY-MM-DD'",
)
@click.option("--deleted", "-d", is_flag=True, help="Includes deleted files from the analysis")
@click.option(
    "--output",
    "-o",
    type=click.Choice([member.value for member in OutputType]),
    default=OutputType.MARKDOWN.value,
    help="Output format",
)
@click.option(
    "--sort",
    type=click.Choice(FIELDNAMES),
    default="hotspot_index",
    help="Sort by the given field",
)
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
def main(
    directory: Path,
    exclude: Sequence[Path],
    output: str,
    deleted: bool,
    sort: str,
    since: str | None,
) -> None:
    since_date = parse_since(since)
    exclude_set = set(exclude)

    maitainability_data = maintainability_index_iter(directory, exclude_set)
    changes_count_data = changes_count_iter(directory, exclude_set, since=since_date)

    metrics = {ROOT_PATH: PathMetrics(path=ROOT_PATH, path_type=PathType.PACKAGE)}

    update_maitainability_metrics(metrics, maitainability_data)
    update_changes_count_metrics(metrics, changes_count_data)

    metrics_iter = get_metrics_iter(metrics.values(), deleted)

    output_type = OutputType(output)

    if output_type == OutputType.CSV:
        print_metrics_csv(metrics_iter)
    elif output_type == OutputType.MARKDOWN:
        print_metrics_markdown(metrics_iter, sort)
    else:
        raise ValueError(f"Unknown output type: {output_type}")


if __name__ == "__main__":
    main()
