# Documentation

このディレクトリには、プロジェクトの設計思想、アーキテクチャ、研究メモなどのドキュメントが含まれています。

## 📐 Architecture

### [transaction-manager-design.md](architecture/transaction-manager-design.md)
Rustにおけるトランザクション管理パターンの最終設計。`Arc<Mutex<DbContext>>` パターンの詳細な解説と実装例。

**主なトピック:**
- Rustの所有権システムとトランザクション管理の相性
- `Arc<Mutex<DbContext>>` パターンの選択理由
- SeaORM / sqlx での実装例
- パフォーマンス考察

### [DESIGN.md](architecture/DESIGN.md)
プロジェクト全体のアーキテクチャ設計。クリーンアーキテクチャとドメイン駆動設計（DDD）の適用方法。

**主なトピック:**
- レイヤー構成（Domain / UseCase / Infrastructure / Application）
- 依存性逆転の原則（DIP）の適用
- ORM非依存な設計
- テスト戦略

## 🔬 Research

### [monad-pattern.md](research/monad-pattern.md)
モナドパターンの調査メモ。Rustにおける関数型プログラミングアプローチの考察。

### [memo.md](research/memo.md)
開発中の気づき、メモ、TODO などを記録したファイル。

## 📦 Archive

過去のバージョンや設計過程を保存しています。現在のコードには直接関係ありませんが、設計の変遷を理解するのに役立ちます。

### [transaction-manager-design-v1.md](archive/transaction-manager-design-v1.md)
初期バージョン：手動トランザクション管理パターン

**主な特徴:**
- `begin()` / `commit()` / `rollback()` の明示的な呼び出し
- 問題点：commit/rollback 忘れのリスク

### [transaction-manager-design-v2.md](archive/transaction-manager-design-v2.md)
第2バージョン：Transaction Block パターン

**主な特徴:**
- クロージャベースのトランザクション管理
- 問題点：Rustの借用チェッカーとの衝突

## 読む順序（推奨）

1. **[../README.md](../README.md)** - プロジェクト概要
2. **[architecture/DESIGN.md](architecture/DESIGN.md)** - アーキテクチャ全体像
3. **[architecture/transaction-manager-design.md](architecture/transaction-manager-design.md)** - トランザクション管理の詳細
4. **[archive/transaction-manager-design-v1.md](archive/transaction-manager-design-v1.md)** → **[v2](archive/transaction-manager-design-v2.md)** - 設計の進化過程（オプション）

## 貢献

ドキュメントの改善提案は Issue または Pull Request でお願いします。
