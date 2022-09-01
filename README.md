# oauth-server-rs

**いまは実装ひどいからまだ見るな！！！！！！！！！！！！！！！**

Rust 実装の自作認証・認可サーバー

認証ページ: http://localhost:3001/authorization

email: sadness_ojisan@example.com

pass: sadness_ojisan

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
