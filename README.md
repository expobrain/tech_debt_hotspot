# tech_debt_hotspot

[![Unit testing, formatting & linting](https://github.com/expobrain/tech_debt_hotspot/actions/workflows/testing-formatting-linting.yml/badge.svg)](https://github.com/expobrain/tech_debt_hotspot/actions/workflows/testing-formatting-linting.yml)

A tool to identify hotspots of tech debt in a Python codebase.

⚠️ **WARNING**: The binary of this tool is not signed so on OSX it will raise a warning. See the official [instructions](https://support.apple.com/en-gb/guide/mac-help/mh40616/mac) to allow the execution of unsigned binaries on OSX.

> This tool comes from the concept expressed in this talk <https://youtu.be/w9YhmMPLQ4U>

This tools collects the maintainability index and the number of changes in the repository for each file of the codebase and aggregated for each package and outputs a CSV with:

- **path**: the path of the Python module or package
- **path_type**: the type of the path (module or package)
- **maintainability_index**: the maintainability index of the module or package calulated by using the Visual Studio's [formula](https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning)
- **changes**: the number of changes in the module or package from the version control
- **hotspot_score**: the inverse of the number of changes over the maintainability index

## Language supported

The tool currently supports only Python.

## Usage

```bash
tech-debt-hotspot /path/to/repo
```

## Example

Example of running the tool in its repository:

```bash
$ tech-debt-hotspot .
+---------------------------------+--------------------+-----------------------+-----+---------------------+-----------------------+---------------+--------------------+
| path                            |   halsteads_volume | cyclomatic_complexity | loc | comments_percentage | maintainability_index | changes_count |      hotspot_index |
+---------------------------------+--------------------+-----------------------+-----+---------------------+-----------------------+---------------+--------------------+
| .                               | 430.04211255552906 |                    32 | 338 |  3.6389206869994304 |    35.786787172962356 |            34 |   95.0071316423948 |
| tech_debt_hotspot.py            | 430.04211255552906 |                    32 | 172 |  0.7407407407407408 |    35.786787172962356 |            14 |  39.12058361745668 |
| tests                           | 274.72299342751836 |                    32 | 166 |   4.770017035775128 |      47.6512709022887 |            14 |  29.38011879831641 |
| tests/tech_debt_hotspot_test.py | 274.72299342751836 |                    32 | 166 |   4.770017035775128 |      47.6512709022887 |            11 | 23.084379055820037 |
| tests/__init__.py               |                  0 |                     1 |   0 |                   0 |                 100.0 |             1 |                1.0 |
+---------------------------------+--------------------+-----------------------+-----+---------------------+-----------------------+---------------+--------------------+
```
