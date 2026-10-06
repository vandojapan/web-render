# libprocessingのAviUtl2接続と実機検証

2026-10-07。AviUtl2 2.1.12、固定済みaviutl2-rs、Windows、RTX 4070。

## 実装

`crates/web-render-aux2`から、独立したRustワーカー
`native/web-render-processing-server`へ接続する実験的なネイティブ経路を追加した。
Bevy/wgpuの依存・GPU状態をCEFプロセスから分離する。
libprocessingはワーカーのメインスレッドで一度だけ初期化し、描画と終了も同じスレッドで行う。
Graphicsはオブジェクトとeffect IDごとに保持し、解像度変更で作り直す。保持数の上限は8。
プロジェクト切替・ホスト終了時にはワーカーを終了する。

ワーカーとの通信はJSONヘッダー＋無圧縮RGBAのパイプ。
version、nonce、寸法、バイト数を検証する。途中で失敗した交換はワーカーを停止し、
プロジェクトの再読込を求める。部分的に読み取ったストリームを再利用しない。
バックエンド・実装版・AA/補間・density=1・素材定義・seedをmanifest fingerprintで識別し、
既存キャッシュの時刻・寸法・パラメーター・offlineフラグ・generationと組み合わせる。
manifestを編集した場合はプロジェクトを再読込する。

`web-render.processing.json`があるフォルダをプロジェクトとして選ぶ。
この場合はNode/CEFを起動しない。Luaオブジェクトは従来と同じSDK経由で生成され、
`obj.putpixeldata(..., "rgba")`へ渡される。HTML・p5.jsプロジェクトは引き続きCEF経路を使う。

ネイティブ側のRGBは線形空間でpremultiplyされ、その後sRGBへエンコードされている。
alphaで割る前に線形化し、straight sRGB RGBAへ変換する。
全RGB/alphaバイトの組合せを64 KiBの表へ保存し、半透明フレームごとのpowfを削除した。
既存のCEF用byte除算は使用していない。

## 対応範囲とサンプル

`examples/processing/web-render.processing.json`に8オブジェクトを定義した。
対応する描画レシピは`shapes`、`text`、`image`、`alpha`。
図形は三角形＋円、文字はアウトライン化した文字列、画像はアップロード済み2色画像。
`count`はAviUtl2の「個数」トラック、`seed`と`animated`はmanifestで指定する。
`no_smooth: true`でMSAA Off＋NearestをGraphics生成後・初回描画前に設定する。
出力形式はRGBA8/sRGB、pixel densityは1、上限は3840×2160。

これはp5.jsのJavaScript実行・自動変換機能ではない。
既存MIDIオブジェクトをネイティブ化するには、MIDI時刻と描画命令をこの経路へ渡す
アダプターが別途必要。今回のMIDI回帰テストは既存CEF経路を対象とする。

元の`test.aup2`から作成したコピーは`examples/aviutl2/libprocessing-sample.aup2`。
8オブジェクトを60フレームずつ並べた。元のプロジェクトは変更していない。

## 検証結果

`docs/proofs/libprocessing-aviutl`に生ログ、画像、比較結果、配置情報を保存した。
`verified/preview-result.json`は8ケース各40フレーム、うちウォームアップ後30フレームの測定。
測定はSDKシーン画像取得の総時間で、IPC・AviUtl2合成・GPU待機を含み、PNG保存時間を含まない。
プラグインはdebug＋検証機能、ネイティブワーカーはrelease。
`first_frame_ms`はプローブの最初の画像取得であり、ホストの自動プレビューが
先にGPU資源を準備している場合がある。ワーカー起動・GPU準備の時間とは分けて扱う。

| 1920×1080のケース | 通常中央値 | noSmooth中央値 |
|---|---:|---:|
| 三角形1000＋円1000 | 21.90 ms | 20.51 ms |
| 文字列100個 | 31.27 ms | 31.93 ms |
| 拡大画像 | 17.00 ms | 18.52 ms |

noSmoothだけで常に速くなるとはいえない。IPC・合成・アウトラインのCPU処理も残る。
独立ライブラリ測定とホスト測定を同じ数値として扱わない。
元の671msのMIDIシーンをこのネイティブ経路で再実装して測った結果ではない。

半透明フルHD画像は、表を使う前の`replay/preview-result.json`で95.05ms、
表を使った`verified/preview-result.json`で16.94ms。
別の実行`final/preview-result.json`では21.59msだった。
表示されるRGBAの白色、alpha 64/192は変更前後とも一致した。

描画・シーク・透明度の検証内容:

- 8ケースの出力が空白にならないこと。
- 逆シーク・同一フレームの再描画でRGBA全体が一致すること。
- 動く図形が時刻で変化し、再シークでは同じ画像になること。
- 最近傍拡大画像の赤/青の画素が正確なこと。
- alpha 64/192の白い画像がstraight RGBAのままホストへ届くこと。
- フリーズ・解除後に同じ画像になること。
- 通常/noSmoothの図形・文字・画像6ケースが、先に保存した独立GPU基準画像と全バイト一致すること。
- ワーカー単体の寸法変更、Graphics再利用、エラー応答後の復帰、正常終了。

CEF回帰ではhimawariのDrum/Kaiwai Phrase/Synth Soloと逆シークが成功した。
p5 MIDIサンプルには以前から60/85フレームの再描画差分がある。
今回も同じ差分を検出したが、9枚すべてのPNGファイルが変更前の記録とバイト単位で一致した。
新たな画素差分は増えていない。`p5-regression/baseline-comparison.json`を参照。
ワークスペースのライブラリ16テストとCEFサーバー2テストも成功した。
検証機能を除いたrelease版も通常起動でワーカーの準備・描画・ホスト終了を確認した。
配置したDLLとワーカーのSHA256をビルド成果物と照合した。
集計は`summary.json`、再集計は`scripts/summarize-processing-host.mjs`。

標準PNG出力の自動検証は別記録とする。
このホストではSDKの`output_file`が`CommonEditSection::output_file`未対応を返す。
標準メニューから保存ダイアログまでは開けるが、自動実行環境の前面ウィンドウが
保存ボタンへの入力を遮ることを確認した。通常の出力完了・savingフラグの実機確認は
成功した検証項目として扱わない。SDK取得画像をPNGへ保存した証拠画像と区別する。

## ディスプレイ制約

`display2_host`はWin32の`\\.\DISPLAY2`を確認してから、専用au2ホストだけを起動する。
今回の作業領域は`[0,0,3840,2100]`。主ウィンドウの設定を起動前に配置し、
所有PIDのウィンドウ・ダイアログをこの画面内に収める。別のモニターでは起動しない。
グローバルマウス入力は、位置がDISPLAY2内で、ヒットしたウィンドウが
検証済みの保存ボタンである場合に限る。証拠は各実行の`display.json`。

## ビルドと実行

```powershell
./scripts/build-windows.ps1 -Profile release
au2 -C .aviutl2-runtime.toml prepare:artifacts -p release --force
```

ワーカーの依存は独立したCargo.lockに固定する。
パッケージ・au2設定にはワーカーとlibprocessing/Lygiaのライセンスを含める。
通常インストールは変更せず、`.aviutl2-cli/development`で検証する。

検証専用ビルド:

```powershell
./scripts/build-windows.ps1 -Profile debug -RuntimeDebug
au2 -C .aviutl2-runtime.toml prepare:artifacts -p debug --force
cargo build -p web-render-aux2 --example display2_host
$env:WEB_RENDER_DEBUG_PROJECT = "$PWD/examples/aviutl2/libprocessing-sample.aup2"
$env:WEB_RENDER_DEBUG_NATIVE_ROOT = "$PWD/examples/processing"
$env:WEB_RENDER_DEBUG_OUTPUT = "$PWD/docs/proofs/libprocessing-aviutl/new-run"
$env:WEB_RENDER_DEBUG_MODE = 'native-replay'
target/debug/examples/display2_host.exe .aviutl2-cli/development/aviutl2.exe `
  $env:WEB_RENDER_DEBUG_PROJECT $env:WEB_RENDER_DEBUG_NATIVE_ROOT `
  "$env:WEB_RENDER_DEBUG_OUTPUT/display.json"
```

初回は空のコピーで`native-bootstrap`を実行し、再起動後`native-test`でオブジェクトを作る。
保存したプロジェクトの再検証は`native-replay`。
検証機能は通常ビルドへ含めない。
