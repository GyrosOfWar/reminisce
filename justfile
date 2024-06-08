set shell := ["nu", "-c"]

default:
    @just --list

download-models:
    http get "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten" | save models/text-detection.rten
    http get "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten"| save models/text-recognition.rten

create-config:
    cp reminisce.exampel.json reminisce.json
    echo "DATABASE_URL=sqlite:reminisce.sqlite3?mode=rwc" | save .env 

create-database:
    cargo sqlx migrate

setup:
    just download-models
    just create-config
    just create-database
