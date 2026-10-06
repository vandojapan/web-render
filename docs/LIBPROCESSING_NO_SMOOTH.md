# libprocessing noSmooth 実装と速度検証（2026-10-07）

仕様書の最初の成果物である、独立したlibprocessing拡張・GPU画像検証・CPU読み戻し込みの測定を実装した。
対象コミットは `0b1a55dd2010a7f170b010ac5fceb48aed7f8d9e`。
既存のHTML/CEF経路は継続する。今回のネイティブ経路はAviUtl2にはまだ接続していない。

ソースは [native/libprocessing](../native/libprocessing/NO_SMOOTH.md)、配布可能な差分は
[libprocessing-no-smooth.patch](../native/libprocessing-no-smooth.patch)、生成Cヘッダーは
[processing-no-smooth.h](../native/processing-no-smooth.h)。パッチの逆適用検査を行い、現在のソースと対応することを確認した。
Cargo.lockとBevyの依存コミットを変更していない。

追加したAPIは `graphics_set_antialiasing`、`graphics_set_image_sampling`、`graphics_no_smooth`。
noSmoothはMSAAを4→1サンプルにし、画像のmag/min/mipmapフィルターをNearestにする。
Bevyの自動DebandDitherも無効にする。実機ではディザリングだけで赤255が254になる画素があったため、
元の二色を保つにはこの制御が必要だった。ユーザー指定のblur、bloom、独自シェーダーには介入しない。

設定はGraphicsごとに保持する。画像のGPUテクスチャは共有し、samplerだけをGraphicsごとに分ける。
既存のimage_set_samplerのフィルター処理を再利用し、Repeat/MirrorRepeatなどのwrapを保つ。
Graphicsで補間を明示した場合は個別画像のfilterより優先し、Defaultでは個別画像の設定を継承する。
初回begin_draw・命令記録・flush以後の変更にはInvalidArgumentを返す。noSmoothが片方だけ適用されることはない。

明示設定のあるGraphicsでは、最初の背景色をrender-pass clearで処理する。
これで全画面メッシュの描画を一つ減らし、背景が半透明画像を後から隠す描画順の問題も解消する。
既存の設定未指定の描画経路は保つ。描画命令の途中にある背景は従来のメッシュ処理を使う。
新しい画像補間経路では元画像のalphaをブレンドし、閾値で透明度を二値化しない。

`graphics_readback_completed_raw`も追加した。end_draw/flushで完了した画像を、
追加のApp.updateを実行せずに読み戻す。従来のgraphics_readback_rawは命令をflushする仕様を維持する。
「end_draw→従来readback」で二重flushしないよう、接続時には完了済み読み戻しAPIを使う。

**測定条件と結果**

Windows 11 Pro 26200、Ryzen 7 7700、RAM 31.2 GiB、RTX 4070 Ti SUPER、driver 591.86。
nativeはVulkan/wgpu 29.0.4、releaseビルド。1920×1080と3840×2160で、
三角形1000個＋円1000個、文字列100個、二色の2×1画像を全画面に拡大するシーンを測った。
10フレームのウォームアップ後に各120フレームを測定し、同じ比較をもう一度実行した。
測定中はCargoのビルドと他の検証を並行させていない。

以下は初回の中央値。nativeは命令生成・CPUテッセレーション／GPUへの投入・GPU待機／CPU読み戻しを含む。
GPU単体の時間ではない。PNG保存、IPC、AviUtl2への引き渡しは含まない。

| 解像度 | シーン | 通常設定 | noSmooth | 短縮率：初回／再測定 | noSmooth p95 |
| --- | --- | ---: | ---: | ---: | ---: |
| 1080p | 図形 | 10.69 ms | 9.89 ms | 7.5%／12.4% | 11.89 ms |
| 1080p | 文字 | 18.62 ms | 17.50 ms | 6.0%／11.8% | 20.30 ms |
| 1080p | 画像拡大 | 6.24 ms | 5.04 ms | 19.2%／20.0% | 7.46 ms |
| 4K | 図形 | 15.76 ms | 15.66 ms | 0.6%／2.3% | 17.03 ms |
| 4K | 文字 | 24.01 ms | 22.01 ms | 8.3%／9.5% | 25.39 ms |
| 4K | 画像拡大 | 10.34 ms | 9.23 ms | 10.8%／37.3% | 12.65 ms |

4K画像の再測定は通常設定側の時間が増えており、37%を安定した改善率として扱わない。
4K図形は両測定とも差が小さい。noSmoothは画質を変える選択肢として公開し、通常設定を一律に置き換えない。

noSmooth時のCPUテッセレーション／投入の中央値は、1080pの文字で15.66 ms、
読み戻し・残りのGPU待機は1.58 msだった。4K図形は投入7.59 ms、読み戻し・待機7.77 ms。
文字のCPU処理と大きな画像の転送が残っているため、AA削減だけでは大幅な改善にならない。
別の改善候補は文字レイアウト／グリフメッシュのキャッシュ、読み戻しバッファの再利用／非同期化、
IPCの画像コピー削減。今回実行したのはAA・補間・自動ディザリングの制御、初期背景のclear化、追加flushを避けるAPIである。

初期化は約2.2～2.4秒、Graphics生成と内部ウォームアップは約0.8～0.9秒、
測定シーンの最初の描画は約163～203 ms。これらを定常フレームの時間から分けて記録した。
nativeプロセスの観測ピークWorking Setは約521～615 MiB、Private Bytesは約950～1299 MiB。
CPU負荷は50 ms周期、NVIDIA全体のGPU負荷／メモリは100 ms周期で記録した。
GPU値にはデスクトップなど他のプロセスも含まれる。CPU/GPUの粗い負荷記録を、正確なGPU draw時間とは扱わない。

CEFの同等構成のCanvas2Dシーンも実際のIPCで測った。各60フレームの中央値は、
1080pの図形105.88 ms・文字99.87 ms・画像95.24 ms、4Kの図形194.43 ms・文字186.17 ms・画像153.95 ms。
こちらはJavaScript、CEFのpaint、画像変換／圧縮、IPC、デコードを含むためnativeの列とは測定範囲が異なる。
この差をAviUtl2プラグイン全体の高速化として報告しない。
同じフレームの再要求も確認したが、フレームキャッシュはaux2側にあるため、直接CEFを呼ぶこの測定は再描画になる。
nativeのキャッシュヒット、ホスト側のキャッシュヒット、AviUtl2引き渡しの時間は未測定。

数値、p95、各フレームの段階別時間、初回時間、CPU/GPU負荷、使用メモリ、画素差分は
[comparison.json](proofs/libprocessing/comparison.json) と同ディレクトリのCSVに保存した。
再測定の全12画像は初回と画素単位で一致した。
変更前のタイミング取得にはビルドとの重なりがあったため、速度の判断には使っていない。
変更前画像は設定未指定の画素互換性の検証に使った。

**画像検証とAPI検証**

実機GPUの読み戻しに対して、斜線・円・塗り・輪郭文字のAA Offに中間色がないこと、
Nearest拡大の二色保持、共有画像の設定独立性、Repeat保持、GPU texture IDの共有、
Graphics→画像→Graphics合成、画像alpha 64/192と半透明塗り、記録後設定変更のエラー、
完了済みreadbackが新しい命令を実行しないこと、破棄・再生成後の画像とGPU画像数を確認した。
設定未指定の6画像は、未変更のlibprocessingで取得した画像とSHA-256が一致した。
15枚の検証PNGは [images](proofs/libprocessing/images/edges-off.png) に保存している。

標準文字はParley→Skrifaアウトライン→Lyonメッシュの経路で検証した。
外部でAA済みのビットマップ文字、SDF、独自シェーダーの平滑化は消えない。
標準Camera3dのMSAAには設定が反映されるが、多様な3D素材とパーティクル専用描画の実画像比較は未実施。
WASMの公開関数は追加したが、wasm32ターゲットのビルドとブラウザー実行は未検証。
このcrateはtarget_arch=wasm32でコード全体を条件コンパイルするため、Windowsのcargo checkでは
ラッパー本体の型検査にはならない。検証環境にはwasm32ターゲットを導入していない。
Pythonのバインディング拡張は未実施。

C ABIの公開関数に対し、未知のmodeによるエラーと、次の呼び出しによるエラー状態の置き換えを検証した。
生成Cヘッダーを使う使用例はMSVCのC17／W4／WXでコンパイルした。

nativeとCEFの完全な画素一致は要求していない。Native Linearの画像補間はsRGBをデコードして補間し、
Canvas2Dとは色の扱いが異なる。またフォント選択とテッセレーション／AA規則の差もある。
比較PNGとRGBA平均絶対差・最大差を記録している。p5.js完全互換とは呼ばない。

**AviUtl2接続**

以下は独立ライブラリ実装時の接続方針。
その後、専用RustワーカーとAviUtl2ネイティブ経路を実装した。
現在の対応範囲と実機結果は[接続・検証記録](LIBPROCESSING_AVIUTL2.md)を参照。

1. Rustの専用レンダーワーカーでlibprocessingを初期化し、以降の描画と終了を同じスレッドで行う。
   Bevy Appは一度だけ初期化し、Graphicsを各オブジェクト／設定で保持する。
   CEFとnativeのwgpu・初期化状態を混在させないため、専用プロセスでの分離が候補となる。
2. request nonce、フレーム、時刻、seed、パラメーターを受け取る明示的なコマンドアダプターを用意する。
   p5.jsスクリプトやMIDI素材がこのRust APIで自動的に動くことはない。
   対応するJSランタイムとp5コマンドの変換、MIDIの絶対時刻による再現処理が別途必要。
3. キャッシュキーにbackend・AA・補間・pixel density・実装版・素材／MIDI・時刻・seed・パラメーターを含める。
   設定変更には新しいGraphicsを用い、処理中の画像リースを壊さずに切り替える。
4. `end_draw`後は`graphics_readback_completed_raw`で読み戻す。RGBA8/sRGBの行ピッチは幅×4。
   通常のalpha blendでは、nativeの半透明RGBは線形空間で合成してからsRGBエンコードされている。
   straight alphaにするにはsRGBを線形化→alphaで割る→sRGBへ戻す。
   現在のCEF BGRA用のbyte除算をそのまま流用しない。REPLACE／独自blendには別途alpha契約を定義する。
   HDR/半精度形式は別途変換する。
   AviUtl2のLua側には既存の`obj.putpixeldata(..., "rgba")`契約に合わせて引き渡す。
5. Rust／既存のaviutl2-rsを使い、専用au2開発ホストでpreview・seek・exportを確認する。
   HTML/CSSは引き続きCEFへ送る。PNG標準出力の自動操作には検証環境の制約があり、
   描画・シークの確認と出力完了の確認を分けて記録する。

再現コマンドは `native/libprocessing/NO_SMOOTH.md` と
`scripts/benchmark-libprocessing.ps1`、`scripts/summarize-libprocessing.mjs` を参照。
