mypy:
	poetry run mypy .

test:
	poetry run pytest -x --cov=core --cov=tech_debt_hotspot --cov-fail-under=90

install:
	poetry install --sync
