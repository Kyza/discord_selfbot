docker compose down
git pull
docker compose up -d --build --remove-orphans
timeout /t 5
