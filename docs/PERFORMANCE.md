# CPU描画の最適化（2026-10-06）

以下は最初のCPU最適化時点の記録。残っていた単発待ちへの追加対策は [アプローチ比較と追加実装](LATENCY_APPROACHES.md) を参照。

同じdebug版・同じhimawari素材で、1画像のIPC描画中央値を約700msから30〜37msへ短縮した。
CEFの4096×2304描画面を毎回全変換し、さらに全コピーする処理が主要な負荷だった。
Rustの `PaintFrame` がOnPaintの入力をcallback内だけで借用し、nonce確認とメタデータ読取の後、
要求画像の範囲だけをstraight RGBAへ変換してコピーするよう変更した。

## 同一素材の計測

`verify_himawari` の実CEF CPU描画・IPC応答を計測。各objectは29要求のうち初回setupを除いた28件。
PNGエンコードと画素検査は計測区間の外。生成MIDI、素材、画面寸法、フレーム順は変更前後で同じ。
逆シーク・再要求・再初期化を含め95画像を取得し、フレームキャッシュによる省略は行っていない。
本記録の中央値はソート後の中央要素（偶数件では上側）、p95はceil(件数×0.95)番目を採用している。

| debug素材 | 変更前中央値 | 変更後中央値 | 変更前p95 | 変更後p95 | 中央値の削減 |
|---|---:|---:|---:|---:|---:|
| Drum（64×184） | 701.6ms | 30.2ms | 891.4ms | 95.6ms | 95.7% |
| Kaiwai Phrase（64×72） | 730.9ms | 32.3ms | 833.4ms | 41.9ms | 95.6% |
| Synth Solo（640×144） | 695.9ms | 37.3ms | 776.4ms | 53.3ms | 94.6% |

| release素材 | 変更前中央値 | 変更後中央値 | 変更前p95 | 変更後p95 |
|---|---:|---:|---:|---:|
| Drum | 84.2ms | 33.0ms | 162.4ms | 68.9ms |
| Kaiwai Phrase | 63.3ms | 28.7ms | 95.9ms | 46.0ms |
| Synth Solo | 66.3ms | 33.6ms | 169.5ms | 920.6ms |

変更後の採用値は他のビルド・検証プロセスと並行しない再計測。
変更前releaseは新debug版のビルドと並行したため、release比較値は参考値。
初回の変更後計測も並行処理があったため採用せず、ログは区別して保存した。

releaseのSynth Soloには約0.9〜1秒の単発IPC待ちが残り、p95は改善していない。
追加診断ではCEFの `batch_render` 完了が約15〜42msである一方、対応する往復要求に1秒超の待ちが発生した。
その計測区間の外に残る待ちの原因は未特定。HTTP/2ウィンドウ拡大では解消せず、その変更は戻した。
この変更によって全要求の遅延上限やリアルタイム再生を保証するものではない。

## AviUtl2と画像の検証

au2の専用AviUtl2 2.1.12で `himawari-sample.aup2` の3objectをSDK経由で描画した。
16回の1920×1080シーン取得は中央値115.1ms、p95 230.9ms。
SDK描画とRGBAコピーを含み、warmup・PNGエンコード・画素検査を除く。
変更前ホストログの3object描画は約1994〜2152msだったが、同じタイマーを持たないため厳密な倍率比較はしない。
逆シークを含む全画像検証に合格し、専用ホストは通常終了、CEFはexit 0を確認した。

- nativeの変更前後debug/release PNG計148枚がSHA-256一致。
- AviUtl2の変更前後PNG16枚がSHA-256一致。
- HTMLのCEF回帰PNG5枚がSHA-256一致。エラー後の描画・再初期化・正常停止も成功。
- Rust単体14件（aux2 5件、cef 9件）とworkspace/all-targets/runtime-debugのClippy `-D warnings` が成功。
- 色変換の単体検証は256段階のalpha×256段階の色成分を従来の全画面変換と比較。
  padded stride、RGBA/BGRAメタデータ、切出し境界、短いバッファも検査。

全描画面は9,437,184画素・37,748,736bytes。従来は全変換と再コピーで約75.5MBの中間バッファを作っていた。
要求画像の出力はDrum 47,104bytes、Kaiwai Phrase 18,432bytes、Synth Solo 368,640bytesだけとなる。
CEF自身の描画面は保持するため、そのメモリや描画コストがゼロになるわけではない。

入力はcallback内で借用し、所有する出力Vecの作成完了後にACKを返す。
4096×2304面、先頭64行の予約、フレーム時刻、要求ID、逐次描画、ACK/キャンセル契約を維持する。
alpha 0/255を短絡処理し、中間alphaは従来と同じ整数丸めで逆premultiplyする。
GPU経路のpadded strideは単体検証のみで、実機性能・色は今回の成功範囲に含めない。

## 証拠と再実行

集計と全PNGハッシュは [comparison.json](proofs/performance/comparison.json)。
[before-debug](proofs/performance/before-debug/result.json)、[after-debug-isolated](proofs/performance/after-debug-isolated/result.json)、
[before-release](proofs/performance/before-release/result.json)、[after-release-isolated](proofs/performance/after-release-isolated/result.json)、
[AviUtl2](proofs/performance/aviutl-after/result.json)、[診断](proofs/performance/latency-diagnostic/result.json) を保存した。
配布用debug/releaseは検証専用 `runtime-debug` featureを外して再ビルド済みで、au2の専用環境にも通常debug版を再配置した。
ビルド証拠は [debug](proofs/performance/default-debug-build.log)、[release](proofs/performance/default-release-build.log)、
[au2配置](proofs/performance/au2-default-restore.log)。成果物のハッシュは [artifacts.json](proofs/artifacts.json)。

```powershell
cargo run --locked -p web-render-cef --example verify_himawari -- dist/windows-debug/Plugin/web-render/web-render-cef-server.exe .local-tests/himawari-receiver docs/proofs/performance/recheck-debug
cargo run --locked --release -p web-render-cef --example verify_himawari -- dist/windows-release/Plugin/web-render/web-render-cef-server.exe .local-tests/himawari-receiver docs/proofs/performance/recheck-release
node scripts/compare-render-performance.mjs
```

作業コピーの準備とMIDIの仕様は [HIMAWARI_TEST.md](HIMAWARI_TEST.md)。
計測は片方ずつ実行する。最後の比較コマンドは保存済みbefore/after-isolatedとホスト・HTMLの証拠を照合する。
