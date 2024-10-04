# tech_debt_hotspot

[![Unit testing, formatting & linting](https://github.com/expobrain/tech_debt_hotspot/actions/workflows/main.yml/badge.svg)](https://github.com/expobrain/tech_debt_hotspot/actions/workflows/main.yml)

A tool to identify hotspots of tech debt in a Python codebase.

> This tool comes from the concept expressed in this talk https://youtu.be/w9YhmMPLQ4U

This tools collects the maitainability index and the number of changes in the repository for each file of the codebase and aggregated for each package and outputs a CSV with:

- **path**: the path of the Python module or package
- **path_type**: the type of the path (module or package)
- **maintainability_index**: the maintainability index of the module or package calulated by `radon` (see https://radon.readthedocs.io/en/latest/intro.html#maintainability-index)
- **changes**: the number of changes in the module or package from the version control
- **hotspot_score**: the product of the maintainability index and the number of changes

## Installation

```bash
pip install tech-debt-hotspot
```

## Usage

```bash
tech-debt-hotspot /path/to/repo
```

## Example

Example of running the tool in its repository:

```bash
$ tech-debt-hotspot .
+---------------------------------+-----------+-----------------------+---------------+--------------------+
| path                            | path_type | maintainability_index | changes_count |      hotspot_index |
+---------------------------------+-----------+-----------------------+---------------+--------------------+
| .                               |   package |      38.8587827245847 |            25 |  64.33552017619766 |
| tech_debt_hotspot.py            |    module |      38.8587827245847 |            10 | 25.734208070479063 |
| tests                           |   package |     48.11991142200354 |             9 | 18.703276323747797 |
| tests/__init__.py               |    module |                 100.0 |             1 |                1.0 |
| tests/tech_debt_hotspot_test.py |    module |     48.11991142200354 |             6 | 12.468850882498531 |
+---------------------------------+-----------+-----------------------+---------------+--------------------+
```
