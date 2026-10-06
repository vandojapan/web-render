# 描画遅延へのアプローチと実施記録（2026-10-06）

前回は全画面のBGRA変換と再コピーを要求範囲の処理へ変更し、debug中央値を約700msから30〜37msへ短縮した。
今回は残る単発の待ちを対象に、同じ生成MIDIとhimawariの3objectで比較する。

| 方法 | 期待する効果・制約 | 判断 |
|---|---|---|
| 要求範囲だけの色変換・コピー | 小さな素材でCPU処理と中間メモリを大幅削減 | 前回実施済み、維持 |
| 補助処理を描画スレッドから退避 | 同期OS呼出しによるCEF pump・IPC停止を防ぐ | 実測に基づき採用 |
| 応答を通知で待つ | 5ms間隔のpollingによる追加待ちと二重pumpを除く | 採用 |
| ログ出力量・出力先の見直し | 同期出力やパイプ詰まりを軽減できる | warnのみでも単発待ちが残ったため、今回の改善策には含めない |
| IPCの可逆圧縮 | 透明部分・単色部分の多い画像の転送量を削減 | 実測で効果があり、圧縮しやすい大きな画像に限定して採用 |
| IPC共有メモリ・転送設定変更 | 大画像のコピー削減。ただし寿命・プロセス終了・再利用の設計が必要 | 今回は標準gRPC圧縮を採用。前回HTTP/2ウィンドウ拡大は効果なし |
| GPU取得・描画面の可変サイズ化 | 描画・読出し量を削減できる | 色・alpha・resize同期の追加検証が必要なため保留 |
| 先読み・キャッシュ・バッチ並列化 | 連続再生に有効 | キャッシュ無効化・シーク・フレーム対応の設計が必要なため保留 |
| rAF待ち削減・低解像度プレビュー | 待ち時間や描画量を削減できる | フレーム一致または画質への影響があるため保留 |

## 実装

従来の親プロセス監視は5秒ごとに `System::new_all()` をCEFと同じcurrent-thread Tokio runtimeで実行していた。
計測では初回約550ms、定常時約90〜140msの間、そのスレッドを占有した。
新しい `parent_process.rs` は親PIDだけを必要最小限の項目で更新し、そのOS処理も `spawn_blocking` へ移す。
親終了時のshutdownを維持し、起動時刻の比較で監視開始後のPID再利用も検知する。
shutdown受信側が閉じたら5秒の待機途中でも監視を終了する。

画像取得では `try_recv` → 5ms sleep → CEF pumpを、`rx.recv()` と30秒deadlineの待機へ変更した。
CEFのpumpは元から存在するmain_serverの5msタイマーが同じ初期化スレッドで継続する。
画像コピー後のACK、callbackのDropによる解除とキャンセル、要求ID照合は保持する。

IPCはtonicのgzip応答を交渉し、クライアントで可逆展開する。protobuf・画像寸法・RGBA内容は共通。
既存依存のflate2を使うtonicのgzip featureを有効化し、crateのバージョンは更新していない。
各画像で最大256か所の隣接画素を等間隔に調べ、75%以上が同色であることと、応答内画像の合計64KiB以上を条件にする。
複数画像のうち一つでもこの同色条件に合わなければ、応答全体の圧縮を無効にする。
細かい写真・ノイズ画像で無駄な圧縮を行う可能性を抑えるための推定であり、全素材の圧縮率や高速化を保証する判定ではない。
gzip非対応クライアントにはtonicの標準交渉で無圧縮応答となる。圧縮するのは応答であり、要求の圧縮は有効にしていない。

## 検証方法

`verify_himawari` に任意の第4引数として連続描画数を追加した。
通常95画像に続け、Synth Soloのframe 15/240/450/0を600回巡回し、毎回全RGBAを既存の正解画像と照合する。
時間計測はIPC要求から画像受取まで。PNG保存と画像比較を除き、キャッシュによる描画省略は行わない。
既存の単体14件に、実子プロセスの終了検出・shutdown受信側を閉じた際の監視停止と、小画像・細かい画像・混在バッチの圧縮回避を検証する2件を追加した。
ignoredテスト1件はその子プロセスとして明示起動するfixture。

```powershell
$env:RUST_LOG='info,web_render_cef_server=debug'
cargo run --locked --release -p web-render-cef --example verify_himawari -- dist/windows-release/Plugin/web-render/web-render-cef-server.exe .local-tests/himawari-receiver docs/proofs/latency/recheck 600
node scripts/compare-render-latency.mjs
```

素材とMIDIの準備は [HIMAWARI_TEST.md](HIMAWARI_TEST.md)。
比較スクリプトは保存済みbefore/afterの画像ハッシュ、合否、中央値・p95・p99・最大値・250ms超の件数を集計する。
中央値は偶数件の上側中央要素。計測は他のビルド・画像試験と並行しない。

## 600連続描画の結果

| 条件 | 中央値 | p95 | p99 | 最大 | 250ms超 |
|---|---:|---:|---:|---:|---:|
| 変更前release | 33.4ms | 56.4ms | 833.7ms | 2999.3ms | 18/600 |
| 親監視・通知待ちのみ | 33.3ms | 45.2ms | 1003.2ms | 1943.1ms | 24/600 |
| 上記＋ログwarnのみ | 33.6ms | 46.0ms | 912.4ms | 1930.6ms | 19/600 |
| gzip試作（大きさだけで選択） | 27.8ms | 37.1ms | 45.2ms | 49.4ms | 0/600 |
| 最終release（画素も判定） | 29.3ms | 35.3ms | 43.6ms | 58.3ms | 0/600 |
| 最終release・別起動 | 33.3ms | 36.0ms | 37.7ms | 38.3ms | 0/600 |
| 最終debug | 35.5ms | 57.3ms | 61.7ms | 68.8ms | 0/600 |

親監視の停止時間は減ったが、それだけでは1秒級の待ちは解消しなかった。
例えば [after-release.log](proofs/latency/after-release.log) の要求142は、CEF描画・コピー・ACKを28.3msで終えた後、
IPC往復全体では1918.8msを要した。ログを抑えても再現したため、ログ出力は今回の主な原因として扱わない。
gzipの有無を比較すると単発待ちが消失した。転送経路内部の待ちの詳細原因は未特定だが、
転送量を減らす対策の効果を圧縮・展開時間込みで確認できた。

最終版は3回、合計1800連続描画で250ms超なし。各回とも通常の95画像試験も実施しており、
最終版の画像要求は合計2085件。画像は毎回全RGBAで比較し、元のMIDIや素材の画素を変更していない。
最終3回のPNG各74枚は変更前とSHA-256一致。通常95要求には初回setupがあり、上表の600連続描画とは区別する。
数値はこの生成MIDI・素材・Windows環境の結果であり、4K写真、全DPI、長時間負荷の検証を代替しない。

集計は [comparison.json](proofs/latency/comparison.json)。変更前、親監視のみ、ログ抑制、gzip試作、
最終版3回の結果とPNGハッシュを分けて保存している。
Rust単体16件（補助子プロセスのignored fixture 1件はテストから起動）、全target Clippy `-D warnings` が成功した。

## AviUtl2・HTML・配布物

au2の専用AviUtl2 2.1.12で保存済みhimawariサンプルを再生し、3objectの合成16画像と逆シークを検証した。
すべて成功し、前回のPNG16枚とSHA-256一致。1920×1080のSDK取得は中央値124.4ms、最大176.4msだった。
SDK計測は3objectの描画とRGBAコピーを含み、warmupとPNGエンコードを除く。
前回のシーン中央値115.1msより小さくはなく、今回の主な成果はCEF単体IPCの長い待ちの解消である。
CEF/SDKとも通常終了を確認し、通常利用のAviUtl2インストールは変更していない。

HTMLの実CEF画像、エラー後の描画、再初期化、shutdownも成功し、PNG5枚が前回と一致した。
最終版のCEF PNG222枚と合わせて243枚のPNGハッシュ一致を確認した。
[SDK結果](proofs/latency/aviutl-final/result.json)、[ホストログ](proofs/latency/aviutl-final/host-log.txt)、
[HTML結果](proofs/latency/html-debug/cef-frame-results.txt)、[集計](proofs/latency/comparison.json)。

初回au2実行はPowerShellの `ErrorActionPreference=Stop` がnative stderrのパス警告を停止扱いし、ホスト起動前に終了した。
stderrだけで失敗扱いせず終了コードを確認する実行へ直し、再実行でホスト検証を完了した。
これは描画失敗には含めない。[初回ログ](proofs/latency/au2-runtime.log)、[再実行ログ](proofs/latency/au2-runtime-retry.log)。

配布用debug/releaseは検証用 `runtime-debug` を含めず、専用au2環境も通常debug版へ復元済み。
[debugビルド](proofs/latency/default-debug-build.log)、[releaseビルド](proofs/latency/release-build-final.log)、
[au2復元](proofs/latency/au2-default-restore.log)、[成果物ハッシュ](proofs/artifacts.json)。
[終了・配置照合](proofs/latency/cleanup.json) で専用配置のaux2/CEFハッシュ一致と残存ホスト/CEFが0件であることを確認した。
