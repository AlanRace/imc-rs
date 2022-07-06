docker build \
    --build-arg http_proxy="http://www-proxy1.uni-marburg.de:3128" --build-arg https_proxy="http://www-proxy1.uni-marburg.de:3128" \
    -t maturin . 

docker run \
    --env HTTP_PROXY="http://www-proxy1.uni-marburg.de:3128" --env HTTPS_PROXY="http://www-proxy1.uni-marburg.de:3128" \
    --rm -v $(pwd):/io maturin build --release -m /io/bindings/python/Cargo.toml