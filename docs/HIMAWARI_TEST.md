# himawari_receiver のMIDI描画テスト

2026-10-07追記: サンプルのMIDI時刻をオブジェクト基準へ揃え、タイムライン移動後もノート表示が追従するようにしました。[修正と検証](TIMELINE_MOVEMENT.md)。以下の初期検証記録は変更前のシーン同期動作です。

2026-10-06 (Asia/Tokyo)。指定された [src/objects](https://github.com/sevenc-nanashi/himawari_receiver/tree/378ea63dd998bb579f8d3ca74c19f9560584a6e3/src/objects) を含むリポジトリを `.local-tests/himawari-receiver-upstream` に shallow cloneしました。対象commitは `378ea63dd998bb579f8d3ca74c19f9560584a6e3` です。

原本を保持し、`.local-tests/himawari-receiver` に `src` をコピーしました。3つのobjectとutilsのTypeScriptは変更していません。[ハッシュ記録](proofs/himawari/source-hashes.json) と [検証結果](proofs/himawari/midi-summary.json) で確認しています。

元リポジトリはMITですが、付属の元曲MIDIはREADMEでMIT対象外とされています。描画にはRustで生成したオリジナルMIDIだけを使い、作業コピーの `src/20260608_himawari_receiver.mid` に置きました。クローンした元曲MIDIは上書きせず、配布パッケージにも追加していません。

## 生成したMIDI

[generated-himawari.mid](proofs/himawari/generated-himawari.mid) はSMF format 1、PPQ 480、15 raw tracks、184音、4/4の8小節、14.4秒です。8秒・tick 7680で120 BPMから150 BPMへ変わります。

Tone.jsは先頭のConductorを除くため、オブジェクトが参照するトラックは次の14本になります。markerは同名の空トラックで、MIDIのmarker meta-eventではありません。

| Tone.js index | 名前 | 内容 |
|---|---|---|
| 0 | drums | ドラムの基準トラック |
| 1–2 | sub-cymbal-a / b | サブシンバル、closed hi-hat |
| 3 | drums-unused | offset維持用 |
| 4–5 | cymbal-a / b | シンバル |
| 6–7 | main-drums-a / b | 元コードの36/37/38/39/41に対応する96音 |
| 8 | synth | シンセの基準トラック |
| 9 | synth-unused | offset維持用 |
| 10–11 | synth-a / b | 各16音と64件のpitch bend |
| 12 | Kaiwai Phrase | フレーズの基準トラック |
| 13 | phrase-notes | 24音、8つの区間と末尾のフェード用空白 |

pitch bendは0、+0.5、-0.5、0を繰り返します。元コードのrange 2と合わせ、現在のノートを上下1半音移動させます。`@tonejs/midi` と `midi-file` の両方で読み込み、順序・音数・note duration・テンポ・拍子・ピッチベンドを検査しています。

## 実行環境と接続

CEF Chromium 144のCPU OnPaint、AviUtl2専用au2環境2.1.12、固定aviutl2-rs 0.48.0を使いました。p5は元リポジトリのlockfileと同じ2.3.2、`@tonejs/midi` 2.0.28、`midi-file` 1.2.4です。

元の `vi5` は作者環境への絶対linkなので、テスト設定のaliasで現行Web Render APIに接続しました。元設定の `?mid` transformを保持して `web-render.config.ts` として用意しています。p5は公式の `dist/app.js` を選択します。元曲や上流vi5ランタイムへの置換はしていません。

初回依存最適化でViteがページを再読込し、CEF初期化/描画のタイムアウトを再現しました。[最初の記録](proofs/himawari/initial-startup-error.log)、[空キャッシュでの記録](proofs/himawari/cold-start-error.log)。テスト設定でp5・MIDIパーサー・priority queueを先に最適化し、未作成だった `.vite-cache-complete` から [95要求の起動試験](proofs/himawari/cef-cold/result.json) が合格しました。通常のプラグイン設定全般を変更する処理ではありません。

## 結果

| 対象 | 確認 |
|---|---|
| MIDI | 184音・トラックoffset・テンポ・拍子・pitch bend検査合格 |
| Drum | 64×184、偶数/奇数小節、Flipによる画像変化、逆シーク全RGBA一致 |
| Kaiwai Phrase | 64×72、フレーズ末尾のフェード、区間切替、globalTime参照、逆シーク全RGBA一致 |
| Synth Solo | 640×144、pitch bendの上下/resetを画素位置で検査、Playhead X、globalTime、最後のノートがスクロールアウトした後の透過 |
| debug CEF | 95要求、再初期化後一致、通常shutdown合格 |
| release CEF | 同じ95要求、再初期化後一致、通常shutdown合格 |
| 空キャッシュからのCEF起動 | 事前最適化する設定で95要求と通常shutdown合格 |
| AviUtl2 | 3種類を別レイヤーにSDK配置、1920×1080 / 30fps、16描画と逆シーク全RGBA一致、コピーを通常保存 |
| AviUtl2保存復元 | 別起動で保存済みコピーを再読込し、同じ16描画と逆シーク全RGBA一致、CEF正常終了 |
| Web回帰 | 5件、型チェック・ビルド合格 |
| Rust | 12件、fmt・全target Clippy合格 |

CEFの各objectでは `0,3,9,15,24,30,45,48,51,54,57,60,63,117,120,237,240,243,252,288,336,429,432,450,15,63,240,450,0` を要求しました。15/63/240/450/0の再要求を全RGBAで比較し、さらに再初期化、Flip、Playhead X、globalTime、スクロール後の追加画像を検査しています。

Synthは最後のnote-off直後も過去のノートを半透明で表示する仕様です。frame 450が空でないことは不具合ではありません。frame 480で表示範囲を抜け、完全透過になることを確認しました。

証拠: [debug CEF](proofs/himawari/cef-debug/result.json)、[release CEF](proofs/himawari/cef-release/result.json)、[AviUtl2 SDK](proofs/himawari/aviutl-capture/result.json)、[ホストログ](proofs/himawari/aviutl-capture/host-log.txt)、[AviUtl2 frame 240](proofs/himawari/aviutl-capture/aviutl-himawari-frame-0240.png)。
保存復元: [SDK再読込結果](proofs/himawari/aviutl-replay/result.json)、[再起動ホストログ](proofs/himawari/aviutl-replay/host-log.txt)。
初回と再読込のPNG16枚はSHA-256がすべて一致しました。[比較](proofs/himawari/image-comparison.json)、[全体結果](proofs/himawari/summary.json)。終了後は通常debug/releaseビルドに戻し、au2で通常debug版を再配置しました。[終了・復元](proofs/himawari/cleanup.json)。

AviUtl2起動時の初回プレビューでは各objectに一時的な `Object not initialized yet` が出るため、異なるフレームで準備を待ってから検証します。生成した3つのobj2だけの信頼確認と、今回のサンプルだけの保存確認を、PID・実行ファイル・ダイアログ文言を照合するRust補助ツールで操作しました。

検証ツールにも修正を加えました。複数objectに同じIDを使って別objectのCanvasを再利用した点を修正し、通知streamを閉じてからCEFをshutdownするよう変更しました。実行中EXEの再配置やWebビルド中のdist削除との競合も記録しています。これらの失敗を素材の描画不具合として扱っていません。

## 再現

ルートのWeb依存、ビルド済みのdebug/releaseパッケージが必要です。[ビルド手順](BUILD_WINDOWS.md) を参照してください。

```powershell
git clone --depth 1 https://github.com/sevenc-nanashi/himawari_receiver.git .local-tests/himawari-receiver-upstream
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-himawari-test.ps1
node scripts/validate-himawari-midi.mjs
cargo run --locked -p web-render-cef --example verify_himawari -- dist/windows-debug/Plugin/web-render/web-render-cef-server.exe .local-tests/himawari-receiver docs/proofs/himawari/cef-debug
cargo run --locked --release -p web-render-cef --example verify_himawari -- dist/windows-release/Plugin/web-render/web-render-cef-server.exe .local-tests/himawari-receiver docs/proofs/himawari/cef-release
```

既にクローン済みならcloneを省きます。将来のmainが変わる場合は上記commitをcheckoutして準備してください。`verify_himawari` は診断を完了すると0を返すため、合否は `result.json` の `status` と `checks` で判断してください。

[himawari-sample.aup2](../examples/aviutl2/himawari-sample.aup2) は現在の `.local-tests/himawari-receiver` への絶対パスを保存しています。DrumをX=-650、Kaiwai PhraseをX=-450、Synth SoloをX=350に配置し、frame 0–479で使用します。

SDK試験は [サンプルデバッグ手順](SAMPLE_DEBUG.md) と同じ `runtime-debug` featureと明示的な環境変数を使います。空のコピーは `himawari-bootstrap` → 再起動 → `himawari-capture`、保存済みコピーは `himawari-replay` です。実行中のホスト/CEFを閉じてから再ビルド・再配置してください。

```powershell
$env:WEB_RENDER_RUNTIME_DEBUG='1'
$env:WEB_RENDER_DEBUG_PROJECT=(Resolve-Path examples/aviutl2/himawari-sample.aup2).Path
$env:WEB_RENDER_DEBUG_OUTPUT=Join-Path (Get-Location) 'docs/proofs/himawari/aviutl-replay'
$env:WEB_RENDER_DEBUG_MODE='himawari-replay'
au2 -C .aviutl2-runtime.toml dev --detach -- $env:WEB_RENDER_DEBUG_PROJECT
```

## 範囲

この生成MIDIとパラメーターでの描画試験です。元曲MVとの視覚一致、MIDIの音声合成/再生、出力プラグインによるPNG連番、4K/DPI、長時間性能、任意のMIDI配置の互換性は検証していません。[前回のMIDI Pattern Grid](P5_MIDI_TEST.md) で検出した画像差・時刻境界の問題が解決したという結果でもありません。
