docker compose down
git pull
docker image prune
docker compose up -d --build --remove-orphans
timeout /t 5