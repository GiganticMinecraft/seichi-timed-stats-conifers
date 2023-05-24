# `server`

## 構成について

このシステム (seichi-timed-stats-conifers) は、次のコンポーネント達に分かれています。

```
|- server
  |- ingestor
  |- grpc-server
  |- usecases
  |- domain
  |- infra
    |- ingestion-client
    |- repository-impl
```

- `domain`
  - 当システム上で用いられる主要な概念 (統計量に関するデータ型やデータリポジトリのシグネチャなど) を規定します
- `usecases`
  - 当システムに対する操作を、 `domain` が規定した語彙で表現します
- `ingestor`
  - 上流のデータソースから統計量を引っ張ってきて、当システム上のデータ点を追加するような定期バッチを走らせます
  - **主要なエントリポイントの一つです**
- `grpc-server`
  - 当システムが蓄積したデータを、整地鯖内の他のシステムに提供する gRPC サーバーを走らせます
  - **主要なエントリポイントの一つです**
- `infra/*`
  - `domain` が規定したリポジトリへのアダプターです。

(TODO: 概要図を挿入する)

## 開発にあたって

システムが利用する永続化バックエンドには RDBMS (MariaDB) を想定しており、スキーママイグレーションの管理には Diesel の migration 機能 ([`diesel-migration`](https://docs.rs/diesel_migrations/latest/diesel_migrations/)) を利用しています。

### `diesel_cli` のセットアップ

開発時には、 `diesel_cli` がローカル環境の DB に対して操作を行えるようにしておくことが推奨されます。そのため、次の手順を実行してください。

1. `cargo install diesel_cli` を実行し、`diesel_cli` をインストールしてください。
1. `.env` を作成し、`.env.example` の内容をコピーしてください。

   - Windows + WSL2 + Docker Desktop 環境ではデフォルトの設定で問題ありませんが、環境によっては `DATABASE_URL` を変更する必要があるかもしれません

1. diesel_cli に利用させる DB を立ち上げるため、 `/server` ディレクトリで次のコマンドを実行して下さい。

   ```
   docker compose -p seichi-timed-stats-conifers-dev -f ./dev-db.docker-compose.yaml up -d
   ```

### マイグレーションの追加

マイグレーションを追加したい時は、次のコマンドを実行してください。

```
diesel migration generate <マイグレーション名>
```

マイグレーションをローカル環境で実行したいときは `diesel migration run` を実行してください。
このコマンドを実行すると、マイグレーションをすべて適用した状態のデータベースから `infra/repository_impl/src/schema.rs` が自動生成されます。

詳細は [Diesel のガイド](https://diesel.rs/guides/getting-started) を参照してください。
