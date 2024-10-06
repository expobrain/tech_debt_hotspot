import math
import textwrap
from collections.abc import Mapping, Sequence
from copy import deepcopy
from datetime import date
from pathlib import Path
from typing import Optional
from unittest.mock import MagicMock, patch

import click
import pytest

from tech_debt_hotspot import (
    FIELDNAMES,
    MINIMUM_MAINTAINABILITY_INDEX,
    ROOT_PATH,
    FileChanges,
    PathMetrics,
    PathType,
    changes_count_iter,
    filename_parent_iter,
    get_metrics_iter,
    get_path_type,
    is_excluded,
    maintainability_index_iter,
    parse_since,
    print_metrics_csv,
    print_metrics_markdown,
    update_changes_count_metrics,
    update_maitainability_metrics,
)


class TestPathMetrics:
    class TestHotspotIndex:
        def test_normal_values(self) -> None:
            # arrange
            metrics = PathMetrics(
                path=ROOT_PATH,
                path_type=PathType.PACKAGE,
                changes_count=50,
                maintainability_index=80,
            )

            # act & assert
            assert metrics.hotspot_index == pytest.approx(62.5)

        def test_zero_changes_count(self) -> None:
            # arrange
            metrics = PathMetrics(
                path=ROOT_PATH,
                path_type=PathType.PACKAGE,
                changes_count=0,
                maintainability_index=80,
            )

            # act & assert
            assert metrics.hotspot_index == pytest.approx(0.0)

        def test_zero_maintainability_index(self) -> None:
            # arrange
            metrics = PathMetrics(
                path=ROOT_PATH,
                path_type=PathType.PACKAGE,
                changes_count=50,
                maintainability_index=0,
            )

            # act & assert
            with pytest.raises(ZeroDivisionError):
                metrics.hotspot_index

        @pytest.mark.parametrize(
            "changes_count, maintainability_index, expected",
            [
                pytest.param(-50, 80, -62.5),
                pytest.param(50, -80, -62.5),
            ],
        )
        def test_negative_values(
            self, changes_count: int, maintainability_index: int, expected: float
        ) -> None:
            # arrange
            metrics = PathMetrics(
                path=ROOT_PATH,
                path_type=PathType.PACKAGE,
                changes_count=changes_count,
                maintainability_index=maintainability_index,
            )

            # act & assert
            assert metrics.hotspot_index == pytest.approx(expected)

    class TestIsDeleted:
        @pytest.mark.parametrize(
            "maintainability_index, expected",
            [
                pytest.param(math.inf, True),
                pytest.param(100, False),
                pytest.param(0, False),
            ],
        )
        def test_is_deleted(self, maintainability_index: float, expected: bool) -> None:
            # arrange
            hotspot = PathMetrics(
                path=ROOT_PATH,
                path_type=PathType.PACKAGE,
                maintainability_index=maintainability_index,
            )

            # act & assert
            assert hotspot.is_deleted() == expected


class TestMaitainabilityIndexIter:
    @patch("tech_debt_hotspot.radon.metrics.mi_visit")
    @patch("tech_debt_hotspot.Path.rglob")
    def test_maintainability_index_iter(
        self, mock_rglob: MagicMock, mock_mi_visit: MagicMock
    ) -> None:
        # arrange
        mock_file1 = MagicMock(spec=Path)
        mock_file1.read_text.return_value = "def foo(): pass"
        mock_file1.relative_to.return_value = Path("file1.py")

        mock_file2 = MagicMock(spec=Path)
        mock_file2.read_text.return_value = "def bar(): pass"
        mock_file2.relative_to.return_value = Path("file2.py")

        mock_rglob.return_value = [mock_file1, mock_file2]

        mock_mi_visit.side_effect = [50, 30]

        directory = Path("/some/directory")
        excluded: set[Path] = set()

        # act
        results = list(maintainability_index_iter(directory, excluded))

        # assert
        assert results == [
            PathMetrics(
                path=Path("file1.py"), path_type=PathType.MODULE, maintainability_index=50
            ),
            PathMetrics(
                path=Path("file2.py"), path_type=PathType.MODULE, maintainability_index=30
            ),
        ]

    @patch("tech_debt_hotspot.radon.metrics.mi_visit")
    @patch("tech_debt_hotspot.Path.rglob")
    def test_maintainability_index_below_minimum(
        self, mock_rglob: MagicMock, mock_mi_visit: MagicMock
    ) -> None:
        # arrange
        mock_file = MagicMock(spec=Path)
        mock_file.read_text.return_value = "def foo(): pass"
        mock_file.relative_to.return_value = Path("file.py")

        mock_rglob.return_value = [mock_file]

        mock_mi_visit.return_value = 0

        directory = Path("/some/directory")
        excluded: set[Path] = set()

        # act
        results = list(maintainability_index_iter(directory, excluded))

        # assert
        assert results == [
            PathMetrics(
                path=Path("file.py"),
                path_type=PathType.MODULE,
                maintainability_index=MINIMUM_MAINTAINABILITY_INDEX,
            )
        ]


class TestChangesCountIter:
    @patch("tech_debt_hotspot.sh.git")
    def test_changes_count_iter(self, mock_git: MagicMock) -> None:
        # arrange
        mock_git.return_value = "file1.py\nfile2.py\nfile1.py\nfile3.py\n"

        directory = Path("/some/directory")
        excluded: set[Path] = set()

        # act
        results = list(changes_count_iter(directory, excluded))

        # assert
        expected = [
            FileChanges(filename=Path("file1.py"), changes_count=2),
            FileChanges(filename=Path("file2.py"), changes_count=1),
            FileChanges(filename=Path("file3.py"), changes_count=1),
        ]

        # Assertions
        assert results == expected

    @patch("tech_debt_hotspot.sh.git")
    @pytest.mark.parametrize(
        "git_log_output, expected",
        [
            pytest.param(
                "file1.py\nfile2.py\nfile1.py\nfile3.py\n",
                [
                    FileChanges(filename=Path("file1.py"), changes_count=2),
                    FileChanges(filename=Path("file2.py"), changes_count=1),
                    FileChanges(filename=Path("file3.py"), changes_count=1),
                ],
            ),
            pytest.param(
                "file1.py\nfile1.py\nfile1.py\n",
                [
                    FileChanges(filename=Path("file1.py"), changes_count=3),
                ],
            ),
            pytest.param("", []),
        ],
    )
    def test_changes_count_iter_parametrized(
        self, mock_git: MagicMock, git_log_output: str, expected: Sequence[FileChanges]
    ) -> None:
        # arrange
        mock_git.return_value = git_log_output

        directory = Path("/some/directory")
        excluded: set[Path] = set()

        # act
        results = list(changes_count_iter(directory, excluded))

        # assert
        assert results == expected

    @patch("tech_debt_hotspot.sh.git")
    @pytest.mark.parametrize(
        "git_log_output, exclude, expected",
        [
            pytest.param(
                "file1.py\nfile1.py\nfile1.py\n",
                set(),
                [FileChanges(filename=Path("file1.py"), changes_count=3)],
            ),
            pytest.param(
                "file1.py\nfile2.py\nfile1.py\nfile3.py\n",
                {Path("file2.py"), Path("file3.py")},
                [FileChanges(filename=Path("file1.py"), changes_count=2)],
            ),
        ],
    )
    def test_changes_count_iter_excluded(
        self,
        mock_git: MagicMock,
        git_log_output: str,
        exclude: set[Path],
        expected: Sequence[FileChanges],
    ) -> None:
        # arrange
        mock_git.return_value = git_log_output

        directory = Path("/some/directory")

        # act
        results = list(changes_count_iter(directory, exclude))

        # assert
        assert results == expected

    @patch("tech_debt_hotspot.sh.git")
    def test_changes_count_iter_default_command(self, mock_git: MagicMock) -> None:
        # arrange
        directory = Path("/some/directory")
        exclude: set[Path] = set()

        # act
        list(changes_count_iter(directory, exclude))

        # assert
        mock_git.assert_called_once_with(
            "log",
            "--name-only",
            "--relative",
            "--pretty=format:",
            directory.as_posix(),
            _cwd=directory,
            _tty_out=False,
        )

    @patch("tech_debt_hotspot.sh.git")
    def test_changes_count_iter_with_sice(self, mock_git: MagicMock) -> None:
        # arrange
        directory = Path("/some/directory")
        exclude: set[Path] = set()
        since = date(2023, 10, 1)

        # act
        list(changes_count_iter(directory, exclude, since=since))

        # assert
        mock_git.assert_called_once_with(
            "log",
            "--name-only",
            "--relative",
            "--pretty=format:",
            "--since",
            since.isoformat(),
            directory.as_posix(),
            _cwd=directory,
            _tty_out=False,
        )


class TestFilenameParentIter:
    @pytest.mark.parametrize(
        "input_path, expected",
        [
            pytest.param(
                Path("/a/b/c/file.txt"),
                [
                    Path("/a/b/c/file.txt"),
                    Path("/a/b/c"),
                    Path("/a/b"),
                    Path("/a"),
                    Path("/"),
                ],
            ),
            pytest.param(Path("/file.txt"), [Path("/file.txt"), Path("/")]),
            pytest.param(Path("file.txt"), [Path("file.txt"), Path(".")]),
        ],
    )
    def test_filename_parent_iter(self, input_path: Path, expected: Sequence[Path]) -> None:
        # act
        result = list(filename_parent_iter(input_path))

        # assert
        assert result == expected


class TestGetPathType:
    @pytest.mark.parametrize(
        "filename, expected",
        [
            pytest.param(Path("module.py"), PathType.MODULE),
            pytest.param(Path("directory/package"), PathType.PACKAGE),
            pytest.param(Path("directory/another_module.py"), PathType.MODULE),
        ],
    )
    def test_get_path_type(self, filename: Path, expected: PathType) -> None:
        # act
        result = get_path_type(filename)

        # assert
        assert result == expected


class TestUpdateMaintainabilityMetrics:
    @pytest.mark.parametrize(
        "maintainability_data, expected",
        [
            pytest.param(
                [
                    PathMetrics(
                        path=Path("/a/b/c/file.py"),
                        path_type=PathType.MODULE,
                        maintainability_index=70,
                    )
                ],
                {
                    Path("/"): PathMetrics(
                        path=Path("/"), path_type=PathType.PACKAGE, maintainability_index=70
                    ),
                    Path("/a"): PathMetrics(
                        path=Path("/a"), path_type=PathType.PACKAGE, maintainability_index=70
                    ),
                    Path("/a/b"): PathMetrics(
                        path=Path("/a/b"), path_type=PathType.PACKAGE, maintainability_index=70
                    ),
                    Path("/a/b/c"): PathMetrics(
                        path=Path("/a/b/c"), path_type=PathType.PACKAGE, maintainability_index=70
                    ),
                    Path("/a/b/c/file.py"): PathMetrics(
                        path=Path("/a/b/c/file.py"),
                        path_type=PathType.MODULE,
                        maintainability_index=70,
                    ),
                },
            ),
        ],
    )
    @pytest.mark.parametrize(
        "metrics",
        [
            pytest.param({}),
            pytest.param(
                {
                    Path("/a/b"): PathMetrics(
                        path=Path("/a/b"), path_type=PathType.PACKAGE, maintainability_index=80
                    )
                }
            ),
            pytest.param(
                {
                    Path("/a/b/c/file.py"): PathMetrics(
                        path=Path("/a/b/c/file.py"),
                        path_type=PathType.MODULE,
                        maintainability_index=70,
                    )
                }
            ),
        ],
    )
    def test_update_maitainability_metrics(
        self,
        metrics: Mapping[Path, PathMetrics],
        maintainability_data: Sequence[PathMetrics],
        expected: Mapping[Path, PathMetrics],
    ) -> None:
        # arrange
        # necessary because the function modifies the input
        metrics_copy = deepcopy(dict(metrics))

        # act
        update_maitainability_metrics(metrics_copy, maintainability_data)

        # assert
        assert metrics_copy == expected


class TestUpdateChangesCountMetrics:
    @pytest.mark.parametrize(
        "changes_count_data, expected",
        [
            pytest.param(
                [FileChanges(filename=Path("/a/b/c/file.py"), changes_count=70)],
                {
                    Path("/"): PathMetrics(
                        path=Path("/"), path_type=PathType.PACKAGE, changes_count=70
                    ),
                    Path("/a"): PathMetrics(
                        path=Path("/a"), path_type=PathType.PACKAGE, changes_count=70
                    ),
                    Path("/a/b"): PathMetrics(
                        path=Path("/a/b"), path_type=PathType.PACKAGE, changes_count=70
                    ),
                    Path("/a/b/c"): PathMetrics(
                        path=Path("/a/b/c"), path_type=PathType.PACKAGE, changes_count=70
                    ),
                    Path("/a/b/c/file.py"): PathMetrics(
                        path=Path("/a/b/c/file.py"),
                        path_type=PathType.MODULE,
                        changes_count=70,
                    ),
                },
            ),
        ],
    )
    @pytest.mark.parametrize(
        "metrics",
        [
            pytest.param({}),
            pytest.param(
                {
                    Path("/a/b"): PathMetrics(
                        path=Path("/a/b"), path_type=PathType.PACKAGE, changes_count=0
                    )
                }
            ),
            pytest.param(
                {
                    Path("/a/b/c/file.py"): PathMetrics(
                        path=Path("/a/b/c/file.py"), path_type=PathType.MODULE
                    )
                }
            ),
        ],
    )
    def test_update_maitainability_metrics(
        self,
        metrics: dict[Path, PathMetrics],
        changes_count_data: Sequence[FileChanges],
        expected: Mapping[Path, PathMetrics],
    ) -> None:
        # act
        update_changes_count_metrics(metrics, changes_count_data)

        # assert
        assert metrics == expected


class TestPrintMetricsCsv:
    @pytest.mark.parametrize(
        "metrics, expected",
        [
            pytest.param(
                [
                    PathMetrics(
                        path=Path("/a/b"),
                        path_type=PathType.MODULE,
                        maintainability_index=75.0,
                        changes_count=5,
                    )
                ],
                (
                    textwrap.dedent(
                        """
                        path,path_type,maintainability_index,changes_count,hotspot_index
                        /a/b,module,75.0,5,6.666666666666667
                        """
                    )
                    .strip()
                    .splitlines()
                ),
                id="single_metric",
            ),
            pytest.param(
                [],
                ["path,path_type,maintainability_index,changes_count,hotspot_index"],
                id="empty_metrics",
            ),
        ],
    )
    def test_print_metrics(
        self, metrics: Sequence[PathMetrics], expected: Sequence[str], capfd: pytest.CaptureFixture
    ) -> None:
        # act
        print_metrics_csv(metrics)

        # assert
        actual = capfd.readouterr().out.splitlines()

        assert actual == expected


class TestPrintMetricsMarkdown:
    @pytest.mark.parametrize(
        "metrics, expected",
        [
            pytest.param(
                [
                    PathMetrics(
                        path=Path("/a/b"),
                        path_type=PathType.MODULE,
                        maintainability_index=75.0,
                        changes_count=5,
                    )
                ],
                textwrap.dedent(
                    """
                        +------+-----------+-----------------------+---------------+-------------------+
                        | path | path_type | maintainability_index | changes_count |     hotspot_index |
                        +------+-----------+-----------------------+---------------+-------------------+
                        | /a/b |    module |                  75.0 |             5 | 6.666666666666667 |
                        +------+-----------+-----------------------+---------------+-------------------+
                    """  # noqa: E501
                )
                .strip()
                .splitlines(),
                id="single_metric",
            ),
            pytest.param(
                [],
                textwrap.dedent(
                    """
                        +------+-----------+-----------------------+---------------+---------------+
                        | path | path_type | maintainability_index | changes_count | hotspot_index |
                        +------+-----------+-----------------------+---------------+---------------+
                        +------+-----------+-----------------------+---------------+---------------+
                    """  # noqa: E501
                )
                .strip()
                .splitlines(),
                id="empty_metrics",
            ),
        ],
    )
    @pytest.mark.parametrize("field_name", FIELDNAMES)
    def test_print_metrics(
        self,
        metrics: Sequence[PathMetrics],
        field_name: str,
        expected: Sequence[str],
        capfd: pytest.CaptureFixture,
    ) -> None:
        # act
        print_metrics_markdown(metrics, field_name)

        # assert
        actual = capfd.readouterr().out.splitlines()

        assert actual == expected


class TestIsExcluded:
    @pytest.mark.parametrize(
        "path, excluded, expected",
        [
            pytest.param(Path("/a/b/c/file.py"), {Path("/a/b")}, True, id="path_is_excluded"),
            pytest.param(Path("/a/b/c/file.py"), {Path("/x/y")}, False, id="path_is_not_excluded"),
            pytest.param(
                Path("/a/b/c/file.py"),
                {Path("/a/b"), Path("/x/y")},
                True,
                id="path_is_excluded_among_multiple",
            ),
            pytest.param(Path("/a/b/c/file.py"), set(), False, id="no_exclusions"),
            pytest.param(
                Path("/a/b/c/file.py"), {Path("/a/b/c/file.py")}, True, id="matching_filename"
            ),
            pytest.param(Path("/a/b/c/"), {Path("/a/b/c/")}, True, id="matching_path"),
        ],
    )
    def test_is_excluded(self, path: Path, excluded: set[Path], expected: bool) -> None:
        # Act
        result = is_excluded(path, excluded)

        # Assert
        assert result == expected


class TestParseSince:
    @pytest.mark.parametrize(
        "since, expected",
        [
            pytest.param(None, None, id="since_is_none"),
            pytest.param("2023-10-01", date(2023, 10, 1), id="valid_date"),
        ],
    )
    def test_parse_since(self, since: Optional[str], expected: Optional[date]) -> None:
        # Act
        actual = parse_since(since)

        # Assert
        assert actual == expected

    @pytest.mark.parametrize(
        "since",
        [
            pytest.param("2023-13-01", id="invalid_month"),
            pytest.param("2023-10-32", id="invalid_day"),
            pytest.param("invalid-date", id="invalid_format"),
        ],
    )
    def test_parse_since_fails_invalid(self, since: str) -> None:
        # Act & Assert
        with pytest.raises(click.BadParameter, match="Invalid date format. Use 'YYYY-MM-DD'"):
            parse_since(since)


class TestGetMetricsIter:
    @pytest.mark.parametrize(
        "metrics, deleted, expected",
        [
            pytest.param(
                [PathMetrics(path=ROOT_PATH, path_type=PathType.MODULE)],
                False,
                [],
            ),
            pytest.param(
                [PathMetrics(path=ROOT_PATH, path_type=PathType.MODULE)],
                True,
                [PathMetrics(path=ROOT_PATH, path_type=PathType.MODULE)],
            ),
            pytest.param(
                [
                    PathMetrics(
                        path=ROOT_PATH, path_type=PathType.MODULE, maintainability_index=75.0
                    )
                ],
                False,
                [
                    PathMetrics(
                        path=ROOT_PATH, path_type=PathType.MODULE, maintainability_index=75.0
                    )
                ],
            ),
            pytest.param(
                [
                    PathMetrics(
                        path=ROOT_PATH, path_type=PathType.MODULE, maintainability_index=75.0
                    )
                ],
                True,
                [
                    PathMetrics(
                        path=ROOT_PATH, path_type=PathType.MODULE, maintainability_index=75.0
                    )
                ],
            ),
        ],
    )
    def test_get_metrics_iter(
        self, metrics: Sequence[PathMetrics], deleted: bool, expected: Sequence[PathMetrics]
    ) -> None:
        # Act
        actual = list(get_metrics_iter(metrics, deleted))

        # Assert
        assert actual == expected
