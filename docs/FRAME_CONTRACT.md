# 現在のフレーム同期契約と証拠

要求ID (`RenderRequest.render_nonce`) は正のi32で、バッチ内で一意。
呼出元が採番し、IPC/JS/画像メタデータ/画像応答に同じIDを保持する。
キャプチャnonceはCEF要求ごとのu32で、初期化0・通知1・予約1024以下と分離する。
初期化nonce 0もネイティブACKまで保持する。起動中のオブジェクト登録/cache purgeはその保持を解除しない。
画像キャッシュのキーは要求IDとは独立し、オブジェクト・パラメータ・ホスト時刻等から作る。

1. ホストが渡したcurrentTimeとframerateをそのままJSへ渡す。秒数をフレーム番号から独自に再計算しない。
2. HTMLは別documentでrenderフック、font/img待ち、CSS/WAAPI/SVG同期を行う。
3. メタデータを描画してから、CEF OnPaintの画素を待つ。JSのフック完了だけでは応答しない。
4. CEFはメタデータのnonceと応答IDを確認し、同じOnPaintバッファから画像を切り出す。
5. メタデータ読取は先頭64行まで。切り出し画像にこの領域を含めない。
6. ネイティブで画像をコピーした後に `acknowledge(captureNonce)` をJSへ返す。
   ログ・一覧通知はACKまでキューに保持し、描画領域を上書きしない。異なるnonceのACKは無視する。
7. 欠落/余剰/重複/古いID、欠けた応答、負寸法、短いバッファ、範囲外切り出しはエラー。
8. タイムアウト・キャンセルではcallbackを除去し `cancelCapture` でdocumentを破棄する。
   runtime generationが変わった非同期完了はメタデータを書かない。

CPU取得は検証済み `PaintFrame` でOnPaint入力をcallback内だけ借用し、nonceとメタデータを
確認した後、要求された範囲だけをstraight RGBAへ変換・コピーする。
全画面の中間コピーを省いても、同じOnPaintからの取得とコピー後のACKという順序は維持する。
中間alphaの逆premultiplyは従来と同じ整数丸めで、変更前後の実画像を比較した。
[最適化の計測と画像一致](PERFORMANCE.md)。

追加の遅延対策ではmain_serverの5ms pumpを保ち、render内の二重pumpと応答pollingを
通知待ちへ置き換えた。30秒deadlineとcallback解除・cancelCaptureは維持する。
IPC応答の選択的gzipは画像コピーとACKの後に行い、展開後の全RGBAを比較する。
[計測と検証](LATENCY_APPROACHES.md)。

`HtmlSession.dispose()` 後のセッションは再利用しない。
フックを強制的に中断するAPIはないが、期限後に更新する古いdocumentを次要求から分離する。
任意のJS時計・物理シミュレーション・乱数状態の仮想化は保証しない。

## 実行した画像試験

`examples/html/pages/frame-proof.html` はフレームごとの全面色と左右8bitバーコード、文字を描く。
`verify_frames` はブラウザスクリーンショットではなく、CEFサーバーのIPC応答に含まれるRGBAを検査する。
入力: 320x180、30000/1001fps、オブジェクトID42、非同期フック、日本語/改行/引用符/バックスラッシュ。
要求順: 0,1,30,90,120,120,90,30,1,0,90,1,120,30,0。
debug/releaseで15要求成功。全画素一致を再要求で確認しPNGと結果を保存。
最終debug結果は [proofs/debug/cef-frame-results.txt](proofs/debug/cef-frame-results.txt)。
releaseの通常パス結果は [proofs/release/cef-frame-results.txt](proofs/release/cef-frame-results.txt)、
EXE/CEF一式とWebプロジェクトの日本語・空白パス結果は [proofs/path-smoke/cef-frame-results.txt](proofs/path-smoke/cef-frame-results.txt)。
最後の二つの実行は並行して起動し、それぞれのポート・プロファイルで画像が一致した。
存在しないシーンの要求ID付きエラー、直後の正常画像、プロジェクト再初期化後の画像も確認。
最終debug/日本語パス試験ではshutdown後10秒以内のexit 0を確認し、強制killを成功扱いしない。

この結果の範囲はこの素材・CEF CPU経路・環境に限る。
さらにAviUtl2 SDKの実タイムライン画像で0→90→30→90→120→1→0を検査し、Freezeと別起動での保存復元も合格した。
初回と再起動後の5枚はPNG全体のSHA-256が一致した。[サンプルデバッグ記録](SAMPLE_DEBUG.md) を参照。
このホスト画像は1920×1080内の320×180素材で、出力プラグインによるPNG連番試験ではない。
2回のrAFを待つ仕組みは残っており、iframeと親canvasのcompositor同期を全素材で証明したものではない。
CSS/WAAPI/SVG、遅い準備、HTML/p5混在、1080p/4K/DPI、AviUtl2 PNG連番の画像試験は未完了。
ready API、素材revision付きgeneration、更新中の一覧通知再送、強制終了後の自動復旧も追加検証が必要。

## p5/MIDIの追加検証

2026-10-06、指定されたMIDI Pattern Gridでp5画像も64行の下へ配置する修正を行った。
非同期setupの二重呼出しとプレビューのPromise参照も修正し、Web回帰で確認した。
生成MIDIのCEF画像・AviUtl2 SDK画像のセル配置は確認したが、逆シーク後の全RGBAに差がある。
このp5素材ではフレーム再現性の受入条件を満たしていない。[詳細](P5_MIDI_TEST.md)。
