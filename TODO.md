# POST.md 改善TODO

## 緊急度：高（技術的正確性の修正）

### 1. async記法の修正 (重要)
**場所**: L415-416 SeaORM実装部分
**現在**:
```rust
async fn transaction<T, F, Fut>(&self, f: F) -> Result<T, Self::Error>
```
**修正後**:
```rust
fn transaction<T, F, Fut>(&self, f: F) -> impl Future<Output = Result<T, Self::Error>> + Send
```
**理由**: trait定義と実装の不一致。読者に誤解を与える

### 2. TransactionManager traitの#[allow]アトリビュート追加
**場所**: L249 TransactionManager trait定義
**追加**:
```rust
#[allow(async_fn_in_trait)]
pub trait TransactionManager {
```
**理由**: 実際のコードとの一致

## 緊急度：中（内容の改善）

### 3. unsafeの正当性説明の強化
**場所**: L86-95
**改善内容**:
- `'static`ライフタイム要求の具体的理由を追加
- transmuteの安全性保証について詳しく説明
- なぜArc<Mutex<T>>でTが'staticである必要があるかの説明

### 4. Arc<Mutex>パターンのパフォーマンス考慮を追加
**場所**: L445-449周辺
**追加内容**:
```
#### パフォーマンス考慮事項
- 非同期環境でのMutexコンテンション
- 参照カウンタのオーバーヘッド
- 高頻度アクセス時の影響
```

### 5. 冗長性の削減
**場所**: L54-95（要件整理部分）
**改善**: 重複する要件説明を整理統合

## 緊急度：低（構成と表現の改善）

### 6. SeaORM実装の位置づけ修正
**場所**: L405-407
**現在**: "## SeaORM 実装: 参考実装"
**修正**: "## SeaORM 実装"
**理由**: 突然「参考実装」扱いにする理由が不明確

### 7. 代替手法の比較追加
**場所**: L494周辺（実装のポイント章）
**追加内容**:
- 他のアプローチ（例：borrowパターン、コールバック型）との比較
- なぜArc<Mutex>を選んだかの理由

### 8. 本番運用での考慮事項追加
**場所**: L538-544（今後の展望）
**追加内容**:
- スケーラビリティの考慮
- 監視・デバッグのポイント
- エラーハンドリングのベストプラクティス

### 9. 「ゼロコスト抽象化」の表現修正
**場所**: L518
**修正理由**: Arc<Mutex>のオーバーヘッドがあるため「ゼロコスト」は不正確
**修正案**: "効率的な抽象化"または"実用的な抽象化"

### 10. テストの価値提案の改善
**場所**: L442-481（テスト戦略）
**改善内容**:
- 単純な数の強調ではなく、テストの質や網羅性を強調
- 各テストカテゴリの意義を説明

## 追加検討事項

### 11. コード例の一貫性チェック
- 全てのコード例で実際のファイルとの整合性を再確認
- importやエラーハンドリングの一貫性確保

### 12. 読者の想定レベルの明確化
- Rust中級者向けなのか、DB設計経験者向けなのかを明確にし、それに応じた説明レベルに調整

---
優先順位: 1-2 > 3-5 > 6-12

## コードベース整理

### 13. transaction_manager_v1.rs, v2.rs のリネーム・移動
**場所**: `crates/infrastructure/repository/sqlx_impl/src/`
**現在**:
- `transaction_manager_v1.rs` - 説明用の古いバージョン
- `transaction_manager_v2.rs` - 説明用の古いバージョン

**タスク**:
- examples/ ディレクトリを作成
- `transaction_manager_v1.rs` → `examples/transaction_manager_pattern1.rs` にリネーム・移動
- `transaction_manager_v2.rs` → `examples/transaction_manager_pattern2.rs` にリネーム・移動
- lib.rs から examples モジュールとして公開
- 各ファイルの冒頭に説明コメントを追加（「教育目的のコード例。本番環境では使用しないこと」など）

**理由**: 
- 説明用のコードであることを明確化
- 本番用のコード（transaction_manager.rs）との区別を明確にする
- examples という命名により、use_case/src/examples と統一感を持たせる
