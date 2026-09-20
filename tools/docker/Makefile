DOCKER_SERVICE := asterinas

.PHONY: run
run:
	@docker compose ps --status running -q $(DOCKER_SERVICE) | grep -q . \
	  || docker compose up -d
	docker compose exec $(DOCKER_SERVICE) bash

.PHONY: stop
stop:
	docker compose stop
