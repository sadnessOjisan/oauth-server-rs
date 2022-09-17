# oauth-server-rs

Rust 実装の自作認証・認可サーバー

```
# http://127.0.0.1:3000/
# 連携するかの確認画面

# http://127.0.0.1:3000/redirected
# 認可コードを受け取るためにリダイレクトで戻される画面

cargo run -p request-app

# http://127.0.0.1:3001/authorization
# ID/PASS を打ち込んで認証できるUIを返す画面
# 作成済みアカウント -> ID: sadness_ojisan@example.com / PASS: sadness_ojisan


# http://127.0.0.1:3001/decide_authorization
# 渡された ID/PASS から認可コードを作り、URLクエリパラメタに認可コードを付けたリクエストアプリにリダイレクトする

# http://127.0.0.1:3001/token_endpoint
# POSTされた認可コードをアクセストークンに引き換えるエンドポイント
# JSON として返す以外にも、今回はクッキーにもつけている

cargo run -p auth-server

# http://127.0.0.1:3002/my_birthday
# アクセストークンに紐づいたユーザー情報を返す（未実装）

cargo run -p resource-server
```

## TODO

- [ ] 認可コードフロー
  - 認可コードを受け取り、トークンエンドポイントでアクセストークンに引き換える
  - URL に含まれる認可コードが漏れても比較的安全
- [ ] implicit flow
- [ ] Resource Owner Password Credentials flow
- [ ] Client Credentials flow
  - トークンエンドポイントにいきなりアクセス
  - I/PASS などを提示してアクセストークンをもらう
- [ ] Refreshing an Access Token flow
