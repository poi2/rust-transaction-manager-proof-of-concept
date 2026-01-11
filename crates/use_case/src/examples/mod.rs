//! トランザクション管理パターンの実例集
//!
//! このモジュールでは、Rustにおけるトランザクション管理の進化を
//! 実際のコード例とともに示します。
//!
//! ## パターンの進化
//!
//! 1. **パターン1: 明示的トランザクション管理** ([`pattern1_manual_transaction`])
//!    - ✅ コンパイル成功
//!    - ❌ commit/rollback忘れのリスク
//!    - ❌ エラー時の自動rollbackなし
//!
//! 2. **パターン2: Transaction Block** ([`pattern2_transaction_block_fails`])
//!    - ❌ コンパイルエラー（E0499）
//!    - ✅ commit/rollback自動管理
//!    - ✅ エラー時の自動rollback
//!    - **Arc/Mutexによる解決が必要**
//!
//! ## 学習の流れ
//!
//! 1. まず[`pattern1_manual_transaction`]を読んで、明示的管理の問題点を理解
//! 2. 次に[`pattern2_transaction_block_fails`]で、より安全な設計への進化と新たな課題を確認
//! 3. 本プロジェクトの実装（`crates/use_case/src/order_management.rs`）で、
//!    `Arc<Mutex<DbContext>>`による解決策を学習

pub mod pattern1_manual_transaction;

#[cfg(feature = "intentional_compile_error")]
pub mod pattern2_transaction_block_fails;
