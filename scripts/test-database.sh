#!/bin/bash

# Database integration test script
# This script starts PostgreSQL in Docker and runs the database tests

set -e

echo "🚀 Starting RodePush Database Integration Tests"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    print_error "Docker is not running. Please start Docker and try again."
    exit 1
fi

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    print_error "docker-compose is not installed. Please install it and try again."
    exit 1
fi

# Stop any existing containers
print_status "Stopping any existing containers..."
docker-compose down -v 2>/dev/null || true

# Start PostgreSQL
print_status "Starting PostgreSQL database..."
docker-compose up -d postgres

# Wait for PostgreSQL to be ready
print_status "Waiting for PostgreSQL to be ready..."
attempts=0
max_attempts=30

while [ $attempts -lt $max_attempts ]; do
    if docker-compose exec -T postgres pg_isready -U rodepush -d rodepush_test > /dev/null 2>&1; then
        print_status "PostgreSQL is ready!"
        break
    fi
    
    attempts=$((attempts + 1))
    print_warning "PostgreSQL not ready yet (attempt $attempts/$max_attempts)..."
    sleep 2
done

if [ $attempts -eq $max_attempts ]; then
    print_error "PostgreSQL failed to become ready within timeout"
    docker-compose logs postgres
    exit 1
fi

# Run database tests
print_status "Running database integration tests..."
cargo test --package rodepush-server --test database_tests -- --nocapture

# Check test results
if [ $? -eq 0 ]; then
    print_status "✅ All database tests passed!"
else
    print_error "❌ Some database tests failed"
    exit 1
fi

# Optional: Keep containers running for manual testing
if [ "$1" = "--keep-running" ]; then
    print_status "Keeping containers running for manual testing..."
    print_status "To stop containers, run: docker-compose down"
else
    # Stop containers
    print_status "Stopping containers..."
    docker-compose down
    print_status "✅ Database integration tests completed successfully!"
fi 