#!/bin/bash

set -e  # Exit immediately if a command fails

# Start services
echo "Starting Docker services..."
docker compose up -d

# Function to check if PostgreSQL is ready
function wait_for_postgres() {
    echo "Waiting for PostgreSQL to be ready..."
    until docker exec postgres_container pg_isready -U postgres; do
        echo "Waiting..."
        sleep 2
    done
    echo "PostgreSQL is ready!"
}

# Wait for PostgreSQL
wait_for_postgres

run_with_env() {
    local original_path="$(pwd)"  # Store the current directory
    local path="$1"
    local env_var="$2"

    if [[ -z "$path" || -z "$env_var" ]]; then
        echo "Usage: run_with_env <path> <ENV_VAR=value>"
        return 1
    fi

    cd "$path" || return 1  # Change to the target directory, exit if it fails
    export "$env_var"       # Set the environment variable

    diesel migration run    # Run the command

    cd "$original_path" || return 1  # Return to the original directory
}

run_with_env "../crates/services/user_service/" "DATABASE_URL=postgres://postgres:password@localhost/users"
run_with_env "../crates/services/admin_service/" "DATABASE_URL=postgres://postgres:password@localhost/admin_db"
run_with_env "../crates/services/game_service/" "DATABASE_URL=postgres://postgres:password@localhost/game"

echo "Everything setup!"
