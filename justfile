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
	pip install .

install-dev-dependencies:
	pip install -r requirements-dev.txt

setup: venv install-dev-dependencies

test:
	flake8 tests/
	black --check tests/
	pytest tests/

# Prepare for benchmarking; will only work on Linux:
prep-benchmark:
	pip install pyahocorasick
	# Disable turbo-boost for more consistent results.
	echo "1" | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

benchmark:
	pytest benchmarks/
