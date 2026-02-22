.PHONY: all build test docker docker-up

all: build

build:
	cargo build

test:
	cargo test --workspace

docker:
	docker build -f toobarr/Dockerfile -t toobarr .

docker-up:
	docker compose -f toobarr/docker-compose.yml up --build
