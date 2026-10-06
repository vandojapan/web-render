# himawariサンプルのタイムライン移動

2026-10-07、AviUtl ExEdit2 2.1.12、DISPLAY2限定の開発ホストで検証。

対象はユーザー指定の `examples/aviutl2/himawari-sample.aup2` が参照する `.local-tests/himawari-receiver`。
3つのMIDI描画がシーン基準の `ctx.frameInfo.globalTime` を使っていたため、オブジェクトを後方へ移動してもMIDIはシーンの元の位置で進んでいた。移動後の終端側ではMIDIのノートを通り過ぎ、表示が消えた。

`scripts/himawari-object-clock.mjs` をテストプロジェクトのVite設定へ追加した。MIDIの小節位置、Drumの開始時刻による絞り込み、Kaiwai Phraseの区間選択とフェードを、すべて `currentTime` に揃える。Synth Soloも共通の小節位置計算を通して同じ時刻になる。cloneしたソースは変更せず、ロード時に対象3モジュールを変換する。準備スクリプトにも同じ設定を反映した。

`globalTime` / `globalFrame` のレンダラーAPIは引き続きシーン基準。シーン上のMIDI位置に固定したい場合は設定の `himawariObjectClock({ timeBase: "scene" })` を使う。現在のサンプルはオブジェクトの開始からMIDIを再生する。

## 実ホストの検証

ユーザーのサンプルは保持し、専用のコピーで試験した。開始位置0のDrum、Kaiwai Phrase、Synth Soloを、120→600→30→0フレームへ移動。各配置で相対450、432、237、63、15フレームを逆順に取得し、移動前の同じ相対フレームの全RGBAと比較した。

- [修正前](proofs/movement/himawari-before/result.json): 20比較中15不一致。開始120、相対432/450ではKaiwai PhraseとSynth Soloの領域の有色画素数が0になった。
- [修正後](proofs/movement/himawari-after/result.json): 20比較すべて一致。元の区間0〜479と重ならない開始600も一致。
- [修正前ログ](proofs/movement/himawari-before/host-log.txt)、[修正後ログ](proofs/movement/himawari-after/host-log.txt)、各フォルダにPNGを保存。
- 対象時計の変換、Drum開始時刻、シーン同期モード、無関係なモジュールの扱いを含むWebテスト7件、aux2のRustテスト5件成功。
- 通常release版でもサンプルのコピーをGUIで操作。Kaiwai PhraseとSynth Soloを開始600へドラッグし、元の区間外の616フレームで両方のノート表示を確認。[GUI画像](proofs/movement/release-gui/synth-ready.png)。専用ホストの通常releaseプラグインのSHA-256はビルド済みDLLと一致（`2CA51937F5C3117F9DEA9117B7E982B4B881FE66CEBDC19DDFAC79B96EC17442`）。

切り分けとしてHTMLとlibprocessing ImageもGUIで0〜149から90〜239へドラッグした。両方とも移動後の6比較が一致。libprocessingはGUIの再生位置を209フレームへ移動した際のプレビューも描画できた。[HTML](proofs/movement/gui/result.json)、[libprocessing](proofs/movement/native-gui/result.json)。

SDK検証モードは `himawari-movement`。`WEB_RENDER_DEBUG_PROJECT` で指定したサンプルコピーに限って動作し、通常ビルドでは無効。
