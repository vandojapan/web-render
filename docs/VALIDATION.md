# 検証結果と次の確認

検証日: 2026-10-05 (Asia/Tokyo)。基準ZIPから実装を進めたWindows環境での結果です。
対象commit、依存、失敗と修正の記録は [BASELINE.md](BASELINE.md)、再現手順は [BUILD_WINDOWS.md](BUILD_WINDOWS.md)。
作業先には.gitがなく作業commitは未設定。SDK commitとCEFバージョンは基準から変更していません。
初期リリースの全受入試験は未完了です。
2026-10-06の追加 [p5/MIDIテスト](P5_MIDI_TEST.md) では描画を確認し、逆シーク画像差と小数時刻境界の遅れを検出しました。p5素材の全受入条件は合格していません。
同日の [himawari_receiverテスト](HIMAWARI_TEST.md) では生成した184音のMIDIで3種類のobjectを実行し、debug/release CEF各95要求とAviUtl2 SDKの16描画・逆シーク全RGBA一致が合格しました。素材ごとに結果が異なります。
同日の [CPU描画最適化](PERFORMANCE.md) では同一debug版の中央値を約700msから30〜37msへ短縮し、変更前後169枚のPNG一致を確認しました。releaseの単発IPC待ちは残っています。
その後の [遅延対策](LATENCY_APPROACHES.md) で親監視の退避・通知待ち・選択的gzipを実装しました。最終release 2回/debug 1回の計1800連続描画では250ms超が0回、全RGBA一致です。

| 確認 | 結果 | 範囲 |
|---|---|---|
| TypeScript型チェック | 成功 | npmパッケージ全体 |
| Webパッケージビルド | 成功 | tsdownによるESMと型宣言生成 |
| ブラウザテスト | 5件成功 | Chromium 153、下記のケース。CEFとは別 |
| 描画画像の透過 | 確認 | ブラウザPNGで背景RGBA=(0,0,0,0)、矩形=(255,0,102,255) |
| Vite側のフレームメタデータ/通知 | 確認 | nonceの4バイト、native ACKまでの保持、不一致ACK拒否、保留ログの再送 |
| 半透明HTML | 確認 | ブラウザPNGのサンプル画素 alpha=199 |
| Unicode文字列 | 確認 | WebのDOM受け渡し、CEFフレーム素材の日本語/改行/引用符/バックスラッシュ入力。全書体の字形保証ではない |
| Rust単体テスト | 16件成功 | 所有権/予約、Lua入力、cache clear、RGBA/stride/crop/metadata、借用入力の色変換一致、要求ID、Webフォルダ事前検証、親終了監視、圧縮対象選択 |
| Rustチェック | 成功 | fmt、check、全target Clippy -D warnings、--locked debug/releaseビルド |
| パッケージ生成 | 成功 | debug/release aux2・EXE・CEF resources/locales/notice、au2専用環境へのcopy配置 |
| CEF CPU実画像 | 成功 | 320x180、30000/1001fps、15非連続/再要求。色・左右バーコード・再要求の全RGBA一致 |
| CEFエラー/再初期化 | 成功 | 要求ID付きの不存在シーンエラー、直後の正常画像、同じプロジェクトの再初期化 |
| CEF正常停止/並行起動 | 成功 | 最終debug/releaseパス試験の並行起動、それぞれshutdown後10秒以内のexit 0 |
| 空白・日本語のWindowsパス | 成功 | release EXE/CEF一式とWebプロジェクトを双方そのパスへコピーし、画像/エラー/再初期化/終了が合格 |
| AviUtl2読込 | 成功 | 専用開発環境2.1.12のaux2・Luaモジュール登録。修正後の起動ログにERRORなし |
| AviUtl2 obj2/タイムライン画像 | 成功 | SDKで「フレーム検証」を配置、0→90→30→90→120→1→0。1920×1080内の320×180素材 |
| AviUtl2 Freeze/保存済み再起動 | 成功 | SDK保存、別ホストで読込。Freeze 90→30→90のWebP保存/読込、解除後も全RGBA一致、初回と再起動後の5枚一致 |
| AviUtl2設定GUI/通常保存/書き出し | 未実行 | マウス操作、GUI保存、出力プラグインPNG連番は別の受入試験 |

保存した証拠:

- [AviUtl2読込ログ](proofs/aviutl2-load.txt)、[au2配置ログ](proofs/au2-dev-final.log)。通常のAviUtl2インストールは変更していない。
- [debug CEF結果](proofs/debug/cef-frame-results.txt)、[PNG frame 90](proofs/debug/cef-frame-0090.png)、[ログ](proofs/cef-debug.log)。
- [release CEF結果](proofs/release/cef-frame-results.txt)、[PNG frame 90](proofs/release/cef-frame-0090.png)、[ログ](proofs/cef-release.log)。
- [release日本語・空白パス結果](proofs/path-smoke/cef-frame-results.txt)、[PNG frame 90](proofs/path-smoke/cef-frame-0090.png)、[ログ](proofs/cef-path-smoke.log)。
- [サンプルデバッグ記録](SAMPLE_DEBUG.md)、[保存済み再起動結果](proofs/aviutl-sample-replay/result.json)、[実ホストログ](proofs/aviutl-sample-replay/host-log.txt)、[画像比較](proofs/aviutl-sample-replay/image-comparison.json)。

CEFログにはChromium内部のGCM `DEPRECATED_ENDPOINT` が残る。画像試験とexit 0は成功した。
Google関連機能を使う素材の互換性まで確認したログではない。
正常終了はCEF統合試験と空タイムラインのSDK bootstrapで確認した。
編集済みサンプル検証のWM_CLOSEはホストで受理されたが、外部から操作できる確認ウィンドウがなく通常停止を完了できなかった。
保存済みコピーと検証成功を確認し、本セッションの専用ホストだけを停止した。[終了記録](proofs/aviutl-sample-replay/cleanup.json)。
終了前のプロジェクト再読込を検証用workerで試した際のEndDraw例外は、その再読込を除去して解消した。
通常ビルドは検証用featureを含まない。手動GUIによるホスト終了の受入試験は未完了。

テストケース:

1. CSSの時刻0.5秒でx=50px、1.5秒へ進めて0.5秒へ戻したPNGが一致。asyncフック、改行・引用符・日本語パラメータも確認。
2. 範囲外サイズ、NaN、別オリジン、javascript URL、内部ページの再帰読込を拒否。4Kサイズは入力検証上許可。
3. オブジェクトごとのdocument分離、640px viewport、破損画像の拒否、HTML mediaの拒否、renderフックのタイムアウト、iframe破棄。
4. 実際のViteランタイムへprotobuf要求を渡し、ページ描画とフレームメタデータを確認。JS pageerrorはなし。
5. renderフックの期限超過後に古いdocumentが更新されても、破棄済みセッションは描画できず、新しいdocumentへ混入しない。

上記5件のWebテストはPlaywrightのChromiumで実行し、CEF OnPaintの証拠として流用しません。
別途 `verify_frames` がCEF CPU OnPaint由来のIPC RGBAを検査します。
CEF契約の成立範囲と限界は [FRAME_CONTRACT.md](FRAME_CONTRACT.md) に記録。
DPIスケール、半透明/文字端のホスト合成、CSS/WAAPI/SVG/p5混在の実CEF画像は追加検証が必要です。

## Windowsでの受入テスト

1. 別のクリーンWindowsで、固定lockfileと配置スクリプトによるビルド/導入を再現する。この開発環境でのdebug/releaseビルドは成功済み。
2. aux2・CEF実行ファイル・対応ランタイムの配置を使い、AviUtl2からの起動/終了/再起動を確認する。CEF単独の初期化/再初期化/終了は成功済み。
3. HTMLサンプルのobj2生成とパラメータ表示を確認する。日本語タイトル、改行、引用符、バックスラッシュを試す。
4. 0→90→30→90フレームをシークし、90フレームの画像が一致することを確認する。
5. 通常再生と非連続フレーム要求、2つのHTMLオブジェクト、異なる画面サイズ、50フレームのバッチを試す。
6. 透明・半透明部分を色付き背景へ重ね、RGB/BGR入替・黒縁・メタデータ混入がないことを確認する。
7. 1080pと4KでPNG連番を書き出し、プレビューとのフレーム一致を比較する。ブラウザ時刻が正しくてもCEFのcompositorが前フレームを返す可能性を必ず確認する。
8. 素材編集→プロジェクト再読込、Freeze、ディスクキャッシュ、プロジェクト保存/復元を確認する。
9. 素材欠落・JS例外・タイムアウト・CEFプロセス終了時にAviUtl2が待ち続けないことを確認する。
10. CPU時間・メモリ・実際のfpsを計測する。GPU経路を有効にする場合は、色・alpha・読み出し完了も改めて検証する。

## vi5との機能対応

| 機能 | 今回の扱い |
|---|---|
| p5.jsオブジェクト登録 | 上流の経路を保持。実機互換性は未検証 |
| 文字列/テキスト/数値/真偽/色パラメータ | 上流定義とLua変換を保持。数値のキーフレーム参照を修正 |
| obj2自動生成 | プラグイン名/出力先を変更、実ホスト生成・SDK登録・文字初期値を検証 |
| プロジェクトフォルダ保存 | 絶対パスの保存と別ホストでの復元を検証。移動は未検証 |
| RAM/ディスクキャッシュ、Freeze | 所有権修正、generation分離。実ホストでFreezeのWebP保存/読込と通常画像一致を検証 |
| バッチ | 要求は保持し、CEF側で1フレームずつ処理する |
| 開発用更新 | Viteを保持。画像キャッシュの完全自動無効化は未実装 |
| GPU取得 | コードを保持、既定では無効 |
| HTML全体 | iframe直接描画経路を追加 |
| CSS/WAAPI/SVG時間同期 | 時刻設定を追加。CSSはブラウザテスト済み、SVGは要追加検証 |
| 任意JSの自動時間仮想化 | 未実装。renderフックで動画時間から算出する |
| HTML video/audio | v0.1では拒否 |

## 主な変更ファイル

- packages/vi5/src/client/htmlSession.mjs: 同一オリジンHTML、読込待ち、時間同期、iframe破棄。
- packages/vi5/src/client/runtime.ts: HTML登録と単一フレームの直接描画。
- packages/vi5/src/user/html.ts: defineHtmlObject API。
- crates/web-render-cef-server/src/server.rs: 描画を直列化し、CEFのキャプチャ後に次フレームを要求。
- crates/web-render-cef-server/src/render_loop.rs: CPU BGRA→straight RGBA、短いバッファ等のチェック。
- crates/web-render-aux2/src/lib.rs: 実行ファイルの相対配置、名称/ポート分離。
- crates/web-render-aux2/src/script.lua: JSON文字列escape、数値パラメータのキーフレーム参照。

機能別の最新状態は [COMPATIBILITY.md](COMPATIBILITY.md) を参照。
上流の未検証コードも残るため「vi5とほぼ同等」の達成にはWindows受入テストの完了が必要です。
