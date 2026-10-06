# web-render.aux2 試作版

vi5.aux2を基に、ローカルHTMLページ全体をChromiumで描画し、AviUtl2へRGBA画像として返す非公式派生版です。HTML/CSS/SVG/Canvasをページにまとめ、タイトル、モーショングラフィックス、Web UIの映像利用を目指します。

**開発版です。Windows debug/releaseビルド、CEF画像試験に加え、AviUtl2 2.1.12のSDKでサンプルのフレーム描画・Freeze・保存後の再起動読込を確認しました。設定GUI・PNG連番書き出しなど、初期リリースの受入試験は未完了です。**

検証済みの [AviUtl2サンプル](examples/aviutl2/html-sample.aup2) と [デバッグ記録・再現手順](docs/SAMPLE_DEBUG.md) を追加しました。サンプル内のWebフォルダはこの作業環境の絶対パスなので、展開先を移動した場合は設定を更新してください。

指定されたp5/MIDI素材でもCEFとAviUtl2の描画をテストしました。画像配置・非同期setupを修正しましたが、逆シーク画像差と時刻境界の遅れが残っています。[MIDI生成ファイルと検証結果](docs/P5_MIDI_TEST.md) を参照してください。

## 構成

| 部品 | 役割 |
|---|---|
| `crates/web-render-aux2` | AviUtl2汎用プラグイン、Luaモジュール、obj2生成、プロジェクト保存 |
| `crates/web-render-cef` | 描画要求・画像応答のIPCクライアント |
| `crates/web-render-cef-server` | 別プロセスのCEF描画、RGBA取り出し |
| `packages/vi5` | npmパッケージ名は `web-render`。Vite、HTMLフレーム同期、従来のp5経路 |
| `examples/html` | HTML/CSSの透過タイトルサンプル |
| `examples/simple` | 上流由来のp5サンプル。全機能の互換性は未検証 |

## 作成済みの動作

- `defineHtmlObject`でHTMLをカスタムオブジェクトとして宣言する。
- HTMLを同一オリジンのiframeに配置し、Chromiumのネイティブ描画を取り込む。DOMをCanvasに擬似変換する方式ではない。
- `window.aviutl.render(frame, params)`へローカル時間・グローバル時間・フレーム・画面サイズ・パラメータを渡す。
- CSS/Web Animationsをpauseし、currentTimeをフレーム時刻に設定する。SVG SMILにも時刻を設定する。
- フォントとimgの読込、非同期renderフックを待つ。失敗・タイムアウトは描画エラーにする。
- オブジェクトIDごとに別documentを保持する。ビューポート最大4096×2240、3840×2160をサイズ規則上許可。
- 上流のパラメータ種類、obj2生成、キャッシュ、Freeze、プロジェクトフォルダ保存、p5経路を引き継ぐ。
- プラグイン名、CLI名、キャッシュ、スクリプト配置をvi5と区別する。IPC/Viteのポートは起動時に割り当てる。

## Web側の準備

Node.js 24を使用します。展開したweb-renderフォルダで実行します。

```powershell
cd packages/vi5
npm ci
npm run typecheck
npm run build
cd ../../examples/html
npm ci
npm start
```

`/vi5`はプラグイン内部のレンダリングページです。`/pages/title.html`がHTML素材です。ユーザーが制作する `.object.ts` は `src`以下に置きます。CSS/JS/画像は同じViteプロジェクト内から読み込みます。

```ts
import { defineHtmlObject } from "web-render";
export default defineHtmlObject({
  id: "my-title", label: "タイトル", source: "/pages/title.html",
  parameters: { text: { type: "text", label: "文字", default: "こんにちは" } },
});
```

```html
<h1 id="title"></h1>
<script>
window.aviutl = {
  render(frame, params) {
    document.querySelector('#title').textContent = params.text;
    // JavaScript描画は毎回frame.currentTimeから状態を計算する。
  }
};
</script>
```

CSSアニメーションは自動同期します。JavaScriptで前回の状態へ加算する処理、setInterval、requestAnimationFrame、Date.now、Math.randomは自動的には動画時間へ同期しません。乱数のseedや物理シミュレーションのseek/resetは素材側で実装してください。DOM更新によって新たに作ったアニメーションもrenderフック完了後に同期します。

## Windowsのプラグインビルド

Windows x64、Rust stable（MSVC）、Visual Studio C++ Build Tools、CMake、Ninjaを使用します。protocとCEFランタイムはCargoビルドで取得します。CEFと低水準バインディングは144.2.0に揃えて固定しています。詳細は [Windowsビルド手順](docs/BUILD_WINDOWS.md) を参照してください。

```powershell
# web-renderルートから
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1 -Profile release
# 開発環境の準備・起動（debugパッケージ作成後）
au2 prepare:aviutl2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1 -Profile debug
au2 -C .aviutl2-runtime.toml dev --detach
```

ビルドスクリプトは取得したCEFランタイムのバージョンと必須ファイルを検証し、DLL・PAK・DAT・BIN・JSON・locales・CREDITSを同梱します。外部ランタイムは `-CefRuntimeDirectory` で指定できます。Windowsでdebug/releaseのパッケージ作成を検証済みです。

出力は `dist/windows-release/Plugin/web-render` です。開発時は上記のau2配置を使用します。aux2、web-render-cef-server.exe、CEF関連ファイルは同じフォルダに置きます。設定メニューで `examples/html`を選び、生成された「HTMLタイトル」obj2を追加します。初回のスクリプト生成時にAviUtl2の再起動・再読込が必要になる場合があります。このGUI手順は引き続き受入試験が必要です。

aviutl2-cli設定も同梱していますが、CEFランタイムを含む配布物の生成には上記スクリプトを使用してください。

## 今回の制約

- 外部の公開URLを素材として登録する機能はない。同一オリジンのローカルHTMLを対象にする。外部JS/フォント等のネットワーク依存も再現性を保証しない。
- iframeは信頼した自作素材向け。上流CEFのno_sandbox設定を引き継いでおり、信頼境界を提供する設計ではない。
- HTML内のvideo/audioは明示的に拒否。AviUtl2側に別素材として置く。動画・音声のseek/decode同期は将来の追加作業。
- SVG、Canvas、WebGLの取り込み経路はHTMLと共通だが、全てのライブラリの互換性は未検証。
- GPU読み出し経路は上流から残すが既定では無効。CPU OSRのBGRA・premultiplied alphaをstraight RGBAへ変換する変更を追加。CEF上の色・透過は要実機検証。
- JS描画完了後に2回のrequestAnimationFrameを待ち、メタデータを描く。ネイティブ側はバッチ内を1要求ずつ取得し、画像コピー後にACKを返す。通知はACKまで保留する。フレーム識別素材で15要求のCEF RGBA一致を確認したが、すべての素材やAviUtl2書き出しの一致は未検証。契約と限界は [FRAME_CONTRACT.md](docs/FRAME_CONTRACT.md) に記載。
- 読込済みHTMLの編集を自動で反映しても、Rust画像キャッシュは内容の変更をキーに含めない。編集後はプロジェクトを再読込してキャッシュを破棄する。p5のHMRやFreezeも上流からの継承であり、全挙動は要検証。
- 64行を通信用メタデータ領域として使い、最終画像から切り落とす。4096×2304の描画面なので小さな素材でもCPU・メモリコストがある。リアルタイム性能の保証はしない。
- aviutl2-rsは固定commit fc94dbf9のままAPI不整合を修正した。最小ホストはSDKが要求する2.1.11、開発読込の確認は2.1.12。

## 検証

検証結果と次の実機チェックは `docs/VALIDATION.md`を参照してください。
指定リポジトリのDrum/Kaiwai Phrase/Synth Soloに合わせたMIDI生成と実CEF/AviUtl2での描画試験は [HIMAWARI_TEST.md](docs/HIMAWARI_TEST.md) に記録しています。
CPU描画の全画面変換・コピーを要求範囲だけの処理へ変更し、同じdebug版の中央値を約700msから30〜37msへ短縮しました。[性能計測と変更前後の画像比較](docs/PERFORMANCE.md) を参照してください。
さらに親監視・応答待ち・IPC転送を改善し、同一素材の600連続描画でreleaseの最大遅延を約3秒から58msへ削減しました。[アプローチ比較と追加検証](docs/LATENCY_APPROACHES.md)。

```powershell
# ルートのテスト用依存関係を導入
npm ci
npx playwright install chromium
npm test
```

## 出典

libprocessingにGraphics単位のnoSmoothを追加し、実機GPUの画像と1080p／4Kの読み戻し速度を検証しました。
[独立ライブラリの実装・性能測定](docs/LIBPROCESSING_NO_SMOOTH.md)を参照してください。
専用RustワーカーをAviUtl2へ接続し、描画・シーク・透明度を検証しました。
[対応範囲・実ホスト測定・サンプル](docs/LIBPROCESSING_AVIUTL2.md)。

- vi5.aux2: https://github.com/sevenc-nanashi/vi5.aux2 （基準commit: 743e87e8d04bbb844fcebb008a66583ee54a3020）
- aviutl2-rs: https://github.com/sevenc-nanashi/aviutl2-rs （依存commit: fc94dbf9d05df7d527a23ae20b934c9ec4c6662a）
- cef-rs: https://github.com/tauri-apps/cef-rs/blob/dev/README.md

上流MITライセンスをLICENSEに保持しています。配布時の依存ライセンスについてはTHIRD_PARTY_NOTICES.mdを参照してください。
