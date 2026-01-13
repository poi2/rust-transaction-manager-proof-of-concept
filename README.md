# Rust Transaction Manager - Proof of Concept

[日本語版 README はこちら / Japanese README](README.ja.md)

A proof-of-concept implementation exploring transaction management patterns in Rust, applying Clean Architecture with portable implementations for both SeaORM and sqlx.

## Overview

This project explores safe transaction management patterns leveraging Rust's type system and ownership, providing portable implementations that work with both SeaORM and sqlx ORMs.

### Key Features

- 🔒 **Type-safe Transaction Management**: Safe concurrency control using the `Arc<Mutex<DbContext>>` pattern
- 🏗️ **Clean Architecture**: Layered architecture based on Domain-Driven Design (DDD)
- 🔄 **ORM-Agnostic**: Supports both SeaORM and sqlx
- ✅ **Comprehensive Testing**: Coverage across unit, integration, and E2E tests

## Documentation

Detailed documentation is available in the [`docs/`](docs/) directory:

- **[docs/README.md](docs/README.md)** - Documentation index
- **[docs/architecture/transaction-manager-design.md](docs/architecture/transaction-manager-design.md)** - Transaction manager design philosophy
- **[docs/architecture/DESIGN.md](docs/architecture/DESIGN.md)** - Architecture design

## Quick Start

### Prerequisites

- Rust 1.88.0+
- PostgreSQL
- Docker & Docker Compose

### Setup

```bash
# Start database
cargo make docker-up

# Setup database
cargo make db-setup

# Run tests
cargo make test

# Run application (SeaORM)
cargo make run-sea-orm

# Run application (sqlx)
cargo make run-sqlx
```

## Project Structure

```
crates/
├── domain/              # Domain layer (entities, value objects, repository interfaces)
├── use_case/            # Use case layer (business logic)
├── infrastructure/      # Infrastructure layer (repository implementations)
│   └── repository/
│       ├── sea_orm_impl/
│       └── sqlx_impl/
└── application/         # Application layer (DI, entry points)
```

## Development

### Testing

```bash
# Unit tests
cargo make test-unit

# Integration tests (requires DB)
cargo make test-all

# Code quality checks
cargo make check-all
```

### Code Quality

```bash
# Format
cargo make fmt

# Clippy
cargo make clippy

# Check for unused dependencies
cargo make udeps
```

## License

MIT

## References

- [Transaction Manager Design Evolution](docs/archive/transaction-manager-design-v1.md) - Evolution of the pattern
