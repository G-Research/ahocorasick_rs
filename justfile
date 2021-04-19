# Local Variables:
# mode: makefile
# End:
# vim: set ft=make :
#
# To use this file, install Just as per https://github.com/casey/just You can
# then run commands as you would with Makefile, e.g. `just venv`.

# Create the virtualenv.
venv:
	python3 -m venv venv

# Compile extension and install into the venv.
build-dev:
	. venv/bin/activate && pip install .

install-dev-dependencies:
	. venv/bin/activate && pip install -r requirements-dev.txt

test:
	flake8 tests/
	black --check tests/
	. venv/bin/activate && pytest tests/
