# lawscape-api

法令文書検索ツールであるlawscapeを動かすためのAPIサーバーと関連ツールの実装です。

以下の3つのworkspaceで構成されています。

- **提供するデータ型や検索エンジンとの接続を行うライブラリ**：[lawscape-core](./lawscape-core/)
- **検索エンジンへの登録用ツール**：[lawscape-register](./lawscape-register/)
- **APIサーバーの実装**：[lawscape-api-server](./lawscape-api-server/)

# lawscape-register

## インストール

```sh
cargo install --git "https://github.com/lawscape/lawscape-api.git" lawscape-register
```

## 実行

事前に[meilisearch](https://www.meilisearch.com/)を動かしておきます。
また、[listup_law](https://github.com/japanese-law-analysis/listup_law)と[listup_precedent](https://github.com/japanese-law-analysis/listup_precedent)を使ってデータを作成しておきます。

データが容易で来たら適切に定義した環境変数と引数を与え、実行します。これにより、meilisearchへのデータの登録が完了します。

```sh
# 環境変数を読み込み

source ~/.meilisearch.env


# dockerを使ったmeilisearchの実行
# バイナリファイルの実行でもよい

sudo docker run -it -d --name lawscape_search --rm -p $MEILISEARCH_PORT:$MEILISEARCH_PORT -v $(pwd)/meili_data:/meili_data   getmeili/meilisearch:v1.13.0 meilisearch --master-key=$MEILISEARCH_MASTER_KEY


# 登録用スクリプトの実行

lawscape-register --law-folder ~/data/law/20250216 --law-index ~/data/law/index20250216.json --precedent-folder ~/data/precedent/20250219 --precedent-index ~/data/precedent/index20250219.json --date 2025-02-21 --meilisearch-url $MEILISEARCH_URL --meilisearch-master-key $MEILISEARCH_MASTER_KEY
```


## 使用例

これにより、次のようにして検索を行うことができるようになります。

```sh
# "公園"で検索する例

curl -X POST "$MEILISEARCH_URL/indexes/$MEILISEARCH_INDEX/search" -H "Content-Type: application/json" --data-binary '{"q": "公園"}' -H "Authorization: Bearer $MEILISEARCH_MASTER_KEY"

# 精度を高め、関連性の高いものだけを選択
curl -X POST "$MEILISEARCH_URL/indexes/$MEILISEARCH_INDEX/search" -H "Content-Type: application/json" --data-binary '{"q": "公園", "limit": 1000000, "locales": ["jpn"], "rankingScoreThreshold": 0.5 }' -H "Authorization: Bearer $MEILISEARCH_MASTER_KEY"
```

# lawscape-api-server

## インストール

```sh
cargo install --git "https://github.com/lawscape/lawscape-api.git" lawscape-api-server
```

## 実行

meilisearchを起動し、`lawscape-register`によるデータの登録が終わっている状態で次のコマンドを実行します。

```sh
# 環境変数の読み込み
source ~/.meilisearch.env
source ~/.server.env

# 実行
lawscape-api-server --meilisearch--url $MEILISEARCH_URL --meilisearch-master-key $MEILISEARCH_MASTER_KEY --threads $API_SERVER_THREADS --bind "0.0.0.0:$API_SERVER_PORT"
```

## 使用例

次のようにして検索ワードや足切りスコア値、取得料などを指定して検索を行うことができます。

```sh
curl -X GET "localhost:$API_SERVER_PORT/v1/search?word=%E5%85%AC%E5%9C%9&cancel_score=0.5&limit=100"
```

---

(c) 2025 Naoki Kitano (puripuri2100)

[The MIT License](https://github.com/lawscape/lawscape-api/blob/master/LICENSE)
