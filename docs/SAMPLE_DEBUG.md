# サンプルプロジェクトでのデバッグ

2026-10-05、AviUtl2 2.1.12の専用開発環境で実行。Rust / aviutl2-rsのSDKから実際のタイムライン、生成obj2、Lua、CEF、ホスト画像を通した検証です。

## 使用したファイル

元ファイルは隣接する `E:\Downloads\web_render_aux2_prototype\test.aup2`。空の1920×1080 / 30fpsプロジェクトで、Webプロジェクトの保存先が展開フォルダ自体になっていました。

元ファイルは変更せず、[html-sample.aup2](../examples/aviutl2/html-sample.aup2) を作成しました。Webフォルダは `examples/html`、レイヤー0には「フレーム検証」を0〜149フレーム配置しています。保存されたパスはこの作業環境の絶対パスです。

元ファイルのSHA-256: `A5DA04F02F405B988B4277C1953FB24F93B8363015A7B59AE2A0C176F974654B`。[原本の記録](proofs/sample-original-sha256.json) を保存しました。通常利用のAviUtl2には配置していません。

## 再現した不具合と修正

1. **Webフォルダの誤指定**: 元プロジェクトを開くと、展開フォルダにnpmランタイムがなくCEF初期化が失敗しました。[元のエラー](proofs/sample-original-error.txt)。CEFを起動する前にフォルダ・package.json・実行ファイルを検査し、Webフォルダ選択またはnpm installを案内します。無効なプロジェクト読込でも前プロジェクトの状態を破棄します。
2. **テキスト初期値の引用符混入**: `--text` の既定値にJSONの外側の引用符を付けたため、AviUtl2で `"Frame proof"` がそのまま文字になっていました。ホスト同梱lua.txtの仕様に合わせ、エスケープを保持して外側の引用符を除去しました。SDKの値取得と画像で `Frame proof` を確認しています。既に保存済みのユーザーテキストは変更しません。
3. **保存済みプロジェクト起動のタイムアウト**: 初期化情報nonce 0がログ・一覧通知nonce 1で先に上書きされました。[失敗時のログ](proofs/aviutl-sample-fixed/replay-startup-error.txt)。初期化もネイティブACKまで保持し、起動中のオブジェクト登録によるcache purgeでも保持を解除しないよう修正しました。CEFは初期化情報を読んでからACK 0を返します。

検証補助にも問題がありました。SDKの単体Objectエイリアス形式とplugin番号維持を修正しました。また、バックグラウンド検証処理からのプロジェクト再読込でホストの `EndDraw` 例外を再現したため、この終了時再読込を除去しました。通常のWM_CLOSEとホスト自身の保存確認を使います。これらは通常ビルドに含まれない検証用処理です。

## 確認した結果

- AviUtl2 SDKによるオブジェクト生成、obj2登録、文字初期値、ホスト合成画像1920×1080を確認。
- 0 → 90 → 30 → 90 → 120 → 1 → 0の7回を要求。色、左右バーコード、再要求時の全RGBA一致を検査。
- Freeze有効で90 → 30 → 90を要求。ログでWebPの保存とディスクからの再読込を確認し、全RGBAが通常画像と一致。解除後の90も一致。
- SDKでコピーを保存し、別のホスト起動で同じ保存済みプロジェクトを読み込み、同じ描画・Freeze検査が合格。
- 初回作成と再起動後の5枚のPNGはSHA-256がすべて一致。[比較結果](proofs/aviutl-sample-replay/image-comparison.json)。
- Rust単体テスト12件、Webテスト5件、型チェック・Webビルド・fmt・Clippyが成功。

証拠: [初回結果](proofs/aviutl-sample-fixed/result.json)、[再起動結果](proofs/aviutl-sample-replay/result.json)、[frame 90](proofs/aviutl-sample-replay/aviutl-frame-0090.png)。

終了処理修正後の再起動検証でも同じ検査が合格し、EndDraw例外は出ていません。[ホストログ](proofs/aviutl-sample-replay/host-log.txt)。bootstrapではホストとCEFの通常終了を確認しました。編集後はWM_CLOSEを送信できましたが、この実行環境では外部から確認ウィンドウへアクセスできず、保存済みコピーを確認して専用ホストだけを停止しました。[終了記録](proofs/aviutl-sample-replay/cleanup.json)。編集後の手動GUI終了試験は未完了です。

画像はSDKの `rendering_scene_video` のコールバックから直ちにコピーしました。HTML検証素材は中央の320×180 Canvasです。1080pのHTMLページ全体・4K性能・DPI・マウスによる設定GUI・出力プラグインのPNG連番書き出しは、この試験の範囲に含みません。SDKの `save_project_file` はバックアップ形式の保存なので、GUIの通常保存の受入試験も別に必要です。

## 再現手順

ルートでWeb依存を導入・ビルドします。[Windowsビルド手順](BUILD_WINDOWS.md) を参照。別の場所に展開した場合は、元の空プロジェクトから別名のコピーを生成してください。既存の出力やタイムラインのある元ファイルへの上書きは拒否します。

```powershell
cargo run --locked -p web-render-aux2 --example prepare_sample -- E:/Downloads/web_render_aux2_prototype/test.aup2 examples/aviutl2/new-sample.aup2 examples/html
```

SDK検証は明示的なdebug featureと環境変数がある場合だけ実行します。ホストもCEFも終了してから配置してください。

```powershell
$env:WEB_RENDER_RUNTIME_DEBUG = '1'
$env:WEB_RENDER_DEBUG_PROJECT = (Resolve-Path examples/aviutl2/new-sample.aup2).Path
$env:WEB_RENDER_DEBUG_OUTPUT = Join-Path (Get-Location) 'docs/proofs/new-sample-bootstrap'
$env:WEB_RENDER_DEBUG_MODE = 'bootstrap'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1 -Profile debug -RuntimeDebug
au2 -C .aviutl2-runtime.toml dev --detach -- $env:WEB_RENDER_DEBUG_PROJECT
```

初回はobj2を生成して閉じます。再起動して `WEB_RENDER_DEBUG_MODE='capture'` と出力先を指定すると、空レイヤー0に検証オブジェクトを作成・保存して描画します。さらに別起動で `WEB_RENDER_DEBUG_MODE='replay'` を指定すると、保存済みオブジェクトを検証します。同梱の `html-sample.aup2` は既にオブジェクトがあるためbootstrapの次はreplayを使います。検証は明示したコピーと現在のプロジェクトが一致する場合だけ実行します。

SDK保存後はホストの編集済み状態が残るため、終了要求で保存確認が出る場合があります。保存したコピーを確認してホストの確認操作を完了してください。検証用コードはこの確認を回避しません。

検証後は環境変数を解除し、通常のdebug/releaseパッケージを再ビルドします。`runtime-debug` は既定で無効、release用の `-RuntimeDebug` は拒否されます。

```powershell
Remove-Item Env:WEB_RENDER_RUNTIME_DEBUG,Env:WEB_RENDER_DEBUG_PROJECT,Env:WEB_RENDER_DEBUG_OUTPUT,Env:WEB_RENDER_DEBUG_MODE -ErrorAction SilentlyContinue
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1 -Profile debug
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1 -Profile release
```
