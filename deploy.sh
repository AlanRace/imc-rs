# docker run --rm -v $(pwd):/io konstin2/maturin publish -m /io/bindings/python/Cargo.toml -r https://test.pypi.org/simple/

twine upload --repository testpypi target/wheels/*.tar.gz
twine upload --repository testpypi target/wheels/*manylinux*