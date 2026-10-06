# p5js MIDIテスト結果

2026-10-06 (Asia/Tokyo)。ユーザー指定の `E:\Downloads\toybox\p5js` を、今回のテスト素材として使用しました。元のソース、テスト、既存 `public/song.mid` は変更していません。[原本ハッシュ](proofs/p5-midi/source-hashes.json) を記録しています。

対象はREADMEの **MIDI Pattern Grid** と付属のMIDI/レイアウト/ディザテストです。vendorのDrum・himawari素材、MIDI音声の合成/再生は今回の対象に含みません。

## 生成したMIDIと実行環境

付属 `scripts/create-demo-midi.mjs` を作業フォルダ内のコピーで実行しました。

- [generated-song.mid](proofs/p5-midi/generated-song.mid): SMF format 0、1トラック、PPQ 480、256バイト。
- 24音、ノート60〜71の8分音符を2回、各240 tick。120 BPMから2秒地点で150 BPMへ変更、全長5.2秒。
- SHA-256: `FB31D56571F294E033136CA5A2335FCD49CF81F3AC3D454224181EDFA8EF3B34`。
- [MIDI解析結果](proofs/p5-midi/midi-summary.json)。元のパーサーで音高・開始/終了tick・tempo mapを検査しました。

コピーは `.local-tests/p5js` に置き、Git/配布物から除外しています。MIDIオブジェクトのソースは変更せず、設定だけで `vi5` のimportを現行web-render APIに接続しました。上流vendorのvi5ランタイムや `/vi5` 上書きパッチは使っていません。元プロジェクト指定のp5 2.2.1を設定で選択しました。現行Web依存のp5 2.3.4でも逆シークの画像差を再現しており、版の違いだけでは解消しません。

CEFはChromium 144のCPU OnPaint、AviUtl2はau2専用環境2.1.12、SDKは固定aviutl2-rs 0.48.0です。

## 結果

**MIDI生成と描画は成功しましたが、再現性の検査に不合格が残っています。全テスト合格として扱いません。**

| 検査 | 結果 |
|---|---|
| 指定プロジェクトの型チェックと既存単体テスト | 成功、9件 |
| 生成MIDIの解析 | 成功、24音と120→150 BPM |
| Web回帰 | 成功、5件。p5のプレビュー準備完了・setup一回実行・async setup失敗応答・64行境界を追加 |
| Rust | 単体12件、fmt、全target Clippy成功 |
| CEF画像 | 824×200。10回の要求でセル配置・透明境界を確認。Global Sync、時刻境界、再初期化を追加し計13回 |
| Bayerディザ | 以前の行の描画画素が約49.997%へ減ることを確認 |
| 逆シーク/再要求 | **不一致**。p5 2.2.1のCEFでframe 85: 2744画素、60: 4938画素、108: 4968画素が変化 |
| Global Syncと対応するObject Syncの画像 | **不一致**。描画履歴の影響を含むため、同期フラグだけの原因とは断定しない |
| プロジェクト再初期化、CEF正常終了 | 成功。再初期化後frame 60一致、shutdown後exit 0 |
| AviUtl2 obj2生成・再起動登録 | 成功。生成スクリプトの信頼確認も専用ホストで完了 |
| AviUtl2 SDKタイムライン描画 | 1920×1080内の824×200素材、53→0→60→85→108→150→85→60→0の9回。各セルの配置を確認 |
| AviUtl2での再要求画像 | **不一致**。85と60フレームの全RGBA比較に差がある |
| AviUtl2保存・終了 | コピーの通常保存確認を操作して保存/終了。最終の読込検証は編集操作をせず通常終了、CEF正常終了もログで確認 |

証拠: [CEF結果](proofs/p5-midi/cef-p5-2.2.1/result.json)、[release CEF結果](proofs/p5-midi/cef-release/result.json)、[SDK結果](proofs/p5-midi/aviutl-capture/result.json)、[SDKホストログ](proofs/p5-midi/aviutl-capture/host-log.txt)。

画像: [CEF frame 85](proofs/p5-midi/cef-p5-2.2.1/p5-frame-0085.png)、[同フレームの再要求](proofs/p5-midi/cef-p5-2.2.1/repeat-mismatch-0085.png)、[実ホスト frame 85](proofs/p5-midi/aviutl-capture/aviutl-p5-frame-0085.png)。[AviUtl2サンプル](../examples/aviutl2/p5-midi-sample.aup2) は今回の作業フォルダへの絶対パスを保存しています。

## 修正したランタイム不具合

1. p5の画像配置が先頭1行から始まり、ネイティブ側の64行メタデータ領域に重なって拒否されました。[再現ログ](proofs/p5-midi/crop-error.log)。p5のパッキングでも64行を予約し、過大メタデータは拒否するよう修正しました。
2. p5生成直後にsetupを手動実行し、p5自身のsetupと二重に呼ばれていました。非同期setupの後にレンダラーが置き換わり空画像になるケースをブラウザで再現。p5のライフサイクルに任せて一回だけ実行し、setup例外で初期化Promiseをrejectするよう修正しました。

2の修正に伴って生じたプレビュー側のPromise参照の食い違いも解消しました。[失敗時の記録](proofs/p5-midi/aviutl-capture/preview-promise-error.json)。非同期プレビューの初回は `Object not initialized yet` になり得るため、SDK検証は異なるフレームで準備を待ちます。最終試験では初回だけの一時警告があり、その後は9回の描画を完了しました。

## 残る不一致

- **時刻境界**: 元のパーサーで2.8秒が8分音符位置 `11.999999999999998` となり、floorでstep 11になります。正確なstep 12の図形がframe 84では現れず、frame 85で現れます。CEF画像とMIDI解析の両方で再現しました。
- **描画履歴**: 同じ時刻へ戻ると、塗りつぶし/輪郭やディザ行の画素が変わります。CEFとSDKホストで再現しています。p5 2.2.1/2.3.4の双方で起きており、オブジェクトのGraphics再利用とp5描画状態を含む切り分けが必要です。原因をCEFの読み出しだけ、または素材だけと断定していません。

入力時刻をランタイムで勝手に丸めたり、期待画像を差し替えて成功扱いする変更は行っていません。元スクリプトを修正する場合は、時刻の整数近傍の扱いと描画状態の再構築を別々に検証する必要があります。

PNGはSDKまたはCEF IPCの実画像です。出力プラグインによるPNG連番、4K/DPI、音声出力、長時間性能の検証ではありません。

## 再実行

```powershell
# ルートで。指定した外部プロジェクトを読み取り、コピーだけを作成する
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-p5-midi-test.ps1 -Source E:/Downloads/toybox/p5js
npm run check:web
npm run build:web
node --test .local-tests/p5js/tests/*.test.ts
node scripts/validate-p5-midi.mjs .local-tests/p5js docs/proofs/p5-midi
cargo run --locked -p web-render-cef --example verify_p5_midi -- dist/windows-debug/Plugin/web-render/web-render-cef-server.exe .local-tests/p5js docs/proofs/p5-midi/cef-p5-2.2.1
```

`verify_p5_midi` は診断を完了すると終了コード0を返します。不一致の有無は `result.json` の `status`・`reverse_seek_full_rgba`・`boundary_issue` を確認してください。今回のstatusは `issues_detected` です。

SDK検証は [サンプルデバッグ手順](SAMPLE_DEBUG.md) と同じ明示的feature/環境変数を使用します。初回は `WEB_RENDER_DEBUG_MODE='p5-bootstrap'` でスクリプトを生成・終了し、次に専用ホストで生成スクリプトの信頼確認を完了します。新規コピーでは `p5-capture`、既に検証オブジェクトを保存したコピーでは `p5-replay` を使います。

```powershell
$env:WEB_RENDER_RUNTIME_DEBUG = '1'
$env:WEB_RENDER_DEBUG_PROJECT = (Resolve-Path examples/aviutl2/p5-midi-sample.aup2).Path
$env:WEB_RENDER_DEBUG_OUTPUT = Join-Path (Get-Location) 'docs/proofs/p5-midi/aviutl-capture'
$env:WEB_RENDER_DEBUG_MODE = 'p5-replay'
au2 -C .aviutl2-runtime.toml dev --detach -- $env:WEB_RENDER_DEBUG_PROJECT
```

通常のウィンドウ列挙で確認画面が見えない環境向けに、`inspect_host_dialog` は指定PIDの実行ファイルを照合し、GetGUIThreadInfoでそのホストのダイアログを診断します。既定は読み取りのみ。明示したオプションでは今回の生成スクリプトだけの信頼確認、または保存済みp5サンプルだけの保存確認を操作できます。通常利用のAviUtl2には使いません。
