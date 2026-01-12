# Rust Transaction Manager - Proof of Concept

Rustにおけるトランザクション管理パターンの実装例と、クリーンアーキテクチャを適用したサンプルプロジェクト。

## 概要

このプロジェクトは、Rustの型システムと所有権を活用した安全なトランザクション管理パターンを探求し、SeaORMとsqlxの両方で動作するポータブルな実装を提供します。

### 主な特徴

- 🔒 **型安全なトランザクション管理**: `Arc<Mutex<DbContext>>` パターンによる安全な並行制御
- 🏗️ **クリーンアーキテクチャ**: ドメイン駆動設計（DDD）に基づいた階層構造
- 🔄 **ORM非依存**: SeaORMとsqlxの両方をサポート
- ✅ **包括的なテスト**: ユニット、統合、E2Eテストを網羅

## ドキュメント

詳細なドキュメントは [`docs/`](docs/) ディレクトリにあります：

- **[docs/README.md](docs/README.md)** - ドキュメント目次
- **[docs/architecture/transaction-manager-design.md](docs/architecture/transaction-manager-design.md)** - トランザクションマネージャーの設計思想
- **[docs/architecture/DESIGN.md](docs/architecture/DESIGN.md)** - アーキテクチャ設計

## クイックスタート

### 前提条件

- Rust 1.88.0+
- PostgreSQL
- Docker & Docker Compose

### セットアップ

```bash
# データベース起動
cargo make docker-up

# データベースセットアップ
cargo make db-setup

# テスト実行
cargo make test

# アプリケーション実行（SeaORM版）
cargo make run-sea-orm

# アプリケーション実行（sqlx版）
cargo make run-sqlx
```

## プロジェクト構成

```
crates/
├── domain/              # ドメイン層（エンティティ、値オブジェクト、リポジトリインターフェース）
├── use_case/            # ユースケース層（ビジネスロジック）
├── infrastructure/      # インフラ層（リポジトリ実装）
│   └── repository/
│       ├── sea_orm_impl/
│       └── sqlx_impl/
└── application/         # アプリケーション層（DI、エントリーポイント）
```

## 開発

### テスト

```bash
# 単体テスト
cargo make test-unit

# 統合テスト（DB必要）
cargo make test-all

# コード品質チェック
cargo make check-all
```

### コード品質

```bash
# フォーマット
cargo make fmt

# Clippy
cargo make clippy

# 未使用依存関係チェック
cargo make udeps
```

## ライセンス

MIT

## 参考資料

- [Transaction Manager Design Evolution](docs/archive/transaction-manager-design-v1.md) - パターンの進化過程
