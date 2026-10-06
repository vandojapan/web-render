# 初期リリース対象と検証範囲

基準ZIPと `docs/BASELINE.md` に記載した環境での棚卸し。上流由来のコード保持と互換試験合格は区別する。

| 機能 | 実装 | 現在の検証 |
|---|---|---|
| ローカルHTML全体 | Chromium iframeの直接描画 | Webテスト・CEF CPUの320x180 Canvas素材、SDKタイムラインのホスト画像合格 |
| 文字列・複数行テキスト | 定義/IPC/Lua/DOM変換保持 | 日本語・改行・引用符のDOM受渡し合格、ホストGUI未検証 |
| 数値・真偽・色 | 定義/IPC/Lua保持 | ホストのキー/中間点/色GUI未検証 |
| CSS/WAAPI時間指定 | pause/currentTime、render後再同期 | CSS順送り/逆送りPNG一致、WAAPI追加試験待ち |
| SVG SMIL | pauseAnimations/setCurrentTime | 未検証 |
| async render | フックとフォント/img待ち、失敗したdocumentの破棄 | Webの成功/失敗/タイムアウト・期限後更新の分離合格、CEFの非同期フレーム素材合格 |
| Canvas 2D / WebGL / Three.js | HTMLページで描画可能な経路を保持 | Canvas 2Dの実CEFフレーム識別素材合格、WebGL/Three.js未実行 |
| p5 setup/draw | setup一回実行と例外応答、64行予約を修正 | MIDI Pattern GridのCEF/SDK描画・Webプレビュー準備/async失敗合格。逆シークの画素差と時刻境界の遅れを検出。WebGL未実行 |
| オブジェクトごとのdocument | iframeセッションをID別に保持 | 分離/viewport変更/破棄のWebテスト合格 |
| obj2生成/再登録/削除 | 生成処理、text既定値の引用符を修正 | ホスト生成・再起動登録・SDK文字初期値合格。削除/GUIは未検証 |
| 保存/復元 | project_dir保存、誤指定フォルダ事前検証、読込前に旧状態破棄 | SDK保存と別起動の絶対パス復元合格。移動/相対パス/GUI通常保存は未検証 |
| RAMキャッシュ/Freeze | 保持、画像寿命修正 | Rust所有権、実ホストのFreeze WebP保存/読込、解除後の画像一致合格 |
| 素材更新 | Vite経路保持 | revision連動の完全な無効化は未実装 |
| バッチ/シーク/書き出し | 要求IDで照合、CEF側は一要求ずつ取得、native ACK | 異常応答をRustで検証、CEFの15非連続/再要求画像合格。複数要求バッチとホスト書き出し未検証 |
| 起動/停止/再初期化 | 動的ポート、readiness、CEF pump、初期化ACK、個別プロファイル | debug/release CEF再初期化/正常停止、パス、並行起動合格。保存済みホスト起動・bootstrap終了合格。編集済みホスト通常終了/強制終了復旧は未完了 |
| AviUtl2ロード | 固定aviutl2-rsのAPIに適合 | 専用開発環境2.1.12のaux2・Lua登録合格。GUIからのHTML設定/描画は未検証 |
| RGBA透過 | CPU BGRAをstraight RGBAへ変換 | 数値変換のRustテスト合格、合成/文字端の実機試験未実行 |
| GPUキャプチャ | 上流実装保持、既定無効 | 未検証、CPUを基準とする |
| 1080p/4K | 4096x2240内のサイズを許可 | 1920×1080のホスト画像内に320×180素材をSDK検証。1080pページ全体/4K/DPI100/150/200%は未検証 |
| HTML video/audio | 初期リリース対象外、拒否 | Webの拒否試験合格 |
| 外部公開URL | 初期リリース対象外、同一オリジンのみ | WebのURL拒否試験合格 |
| 任意JSの時計/timer/rAF仮想化 | 初期リリース対象外 | 素材側がframe.currentTimeから計算する |

現在の結果と証拠は [VALIDATION.md](VALIDATION.md) を参照。
指定p5素材の不一致を含む結果は [P5_MIDI_TEST.md](P5_MIDI_TEST.md)。
himawari_receiverのDrum/Kaiwai Phrase/Synth Soloは、生成MIDIでCEFとSDKの逆シーク画像一致を確認しました。[範囲と結果](HIMAWARI_TEST.md)。
初期リリースの完成を判断するには、素材別のCEF画像一致とAviUtl2での合成・保存復元・PNG連番が必要。
