# 基準と作業記録

検証日: 2026-10-05 (Asia/Tokyo)。基準は `E:/Downloads/web_render_aux2_prototype.zip`。
SHA-256: `95ab473227537c2e41aca2cbd06ed78f36b892cb8841df4f700fc06a88eb3769`（実測一致）。
作業フォルダはZIP展開物で `.git` はないため、作業ブランチ・commitは存在しない。
派生元commitは `743e87e8d04bbb844fcebb008a66583ee54a3020`。この記録は上流の現在の動作を保証しない。

## 環境

| 項目 | 実測・状態 |
|---|---|
| OS | Windows build 26200.9457 / 25H2 / x64（レジストリのProductNameはWindows 10 Pro） |
| Node / npm | 24.16.0 / 11.13.0 |
| Rust / Cargo | 1.97.1 / 1.97.1 / stable MSVC |
| Visual Studio | 18 Community、C++ツールチェーン検出済み |
| CMake | 4.3.3 |
| protoc | PATHにはない。build.rsでprotoc-bin-vendoredを明示指定 |
| aviutl2-rs | fc94dbf9d05df7d527a23ae20b934c9ec4c6662a / 0.48.0、最小ホスト2.1.11（固定SDKのCHANGELOG） |
| CEF / cef-dll-sys | 144.2.0+144.0.11、両方を固定。ランタイムはcef-dll-sysの取得物 |
| au2 | 0.10.2、既存aviutl2.tomlを使用 |
| AviUtl2 | au2専用開発環境の2.1.12でaux2・Luaモジュール読込成功。通常インストールは変更していない |
| ブラウザ | Playwright Chromium 153.0.8010.12 (v1243)、CEFとは別 |
| CPU / RAM / GPU | AviUtl2ログ: Ryzen 7 7700 / 32768MB / NVIDIA RTX 4070 Ti SUPER |
| DPI | 未測定。100/150/200%の受入試験は未実行 |

## 基準の再実行

依存の取得・キャッシュ書込にはこの実行環境のサンドボックス外実行を使用した。
最初の通常権限のCargo/npm実行はキャッシュ書込・ネットワーク権限で失敗した。

```powershell
npm ci --prefix packages/vi5
npm install
npm ci --prefix examples/html
npx playwright install chromium
npm run check:web
npm run build:web
npm test
```

型チェック・ビルド成功。基準のブラウザテスト4件成功（変更前）。
CSS逆シーク、非同期フック、日本語DOM、iframe分離、エラー拒否、Vite/protobufメタデータを再現。
PNGは `tests/html-render-proof.png` / `tests/protocol-proof.png`。
これらはCEF OnPaintやAviUtl2書き出しの検証ではない。

## ネイティブ側で確認した問題と修正

- 最初の `cargo check` はprotoc不足で失敗。vendored protocを追加し、環境変数をunsafeに変更せずConfigで指定。
- cef=144.2.0だけではcef-dll-sys=144.4.0へ解決されるため、低水準バインディングも=144.2.0に固定。
- 固定aviutl2-rsのログAPI、GenericPluginTable、register_script_moduleに合わせて移行。SDK commitは変更していない。
- Freeze経路の返却元Vecと保持Vecが異なる問題を修正。通常経路も同じ所有権管理を使用。
- 同一effectの未解放画像がある間は次のcallを拒否。失敗した描画の予約は自動解除。
- cache clearは返却画像を解放しない。Luaの画像コピーがエラーになってもfreeを呼ぶ。
- 空/過大バッチ、負寸法、バッファ長、stride、切出し範囲、メタデータ領域の読取を検証。
- 要求IDを呼出元から渡し、結果をIDでキャッシュへ対応付け。重複・余剰・欠落・応答未設定はエラー。
- ルート描画エラーはnonce=0に置き換えず、要求IDへ対応付ける。

`cargo check --locked`、debug/releaseビルド、fmt、Clippy（全target、警告をエラー扱い）成功。
Rustテスト11件成功（画像寿命/予約2、Lua入力1、cache clear中の画像寿命1、画像境界/変換5、応答ID2）。
追加した所有権とunsafe境界は [NATIVE_BOUNDARIES.md](NATIVE_BOUNDARIES.md) に記録。

## 起動・フレーム同期の修正と実行結果

- CEF pumpを初期化と同じthreadのcurrent-thread Tokio上でidle時にも実行。CPU経路はGPUを初期化しない。
- IPC/Viteの動的ポート、HTTP readiness、起動期限と子プロセス終了検出を追加。
- Nodeを直接起動し、Windowsの拡張パス接頭辞を正規化。正常停止時の子終了と親終了監視を追加。
- ホスト終了処理はIPCを所有するTokio runtime上で実行し、server lock待ちも含め12秒で打ち切る。正常終了を待ち、失敗時のみkillへ進み、runtime停止も5秒で制限する。
- プロジェクト通知をgenerationで分離し、旧購読を中止。空の新規プロジェクトはエラーにしない。
- 画像コピー後のnative ACKまでメタデータを保持。保留したログ・一覧通知はACK後に再送。
- 描画失敗/タイムアウト時にcallback・iframeを破棄し、古いasync完了を次要求へ混入させない。
- au2のディレクトリcopy失敗（os error 5）とsymlink権限不足（1314）を、全CEFファイルの個別copy定義で修正。
- au2がcmd.exeへ渡す拡張パスのcwd問題を、絶対スクリプトパスのEncodedCommandで修正。

変更後のWeb型チェック・ビルド・5件のテスト成功。追加ケースは期限後のasync更新のdocument分離。
protocol試験は動的ポートを使い、正しいnative ACK前のメタデータ保持・ログ再送も検査する。
このWeb試験と、次のCEF画像試験は別々の証拠として扱う。

debug/releaseのパッケージを `dist/windows-{debug,release}/Plugin/web-render` に生成。
CEF CPU OnPaintのIPC RGBAで15要求の色・左右バーコード・再要求時の全画素一致を確認。
存在しないオブジェクトの対応付きエラー、その後の正常描画、再初期化、10秒以内のexit 0も成功。
releaseのEXE/CEF一式とWebプロジェクトをそれぞれ空白・日本語を含むパスへコピーして同じ試験が成功。
このrelease試験とdebug試験を並行実行し、動的ポートとCEFプロファイルを分離して動作した。
画像と結果は [proofs/debug](proofs/debug/cef-frame-results.txt)、[proofs/release](proofs/release/cef-frame-results.txt)、
[proofs/path-smoke](proofs/path-smoke/cef-frame-results.txt)。最終lintの一時的な自動承認レビュー容量エラーは再試行で解消。

AviUtl2 2.1.12ではaux2ロードと `web-render` Luaモジュール登録をログで確認。
保存先未設定をERRORにしていた初回ロードを修正し、再配置後の起動ログにERRORはない。
[読込ログ](proofs/aviutl2-load.txt) はGUI描画・PNG連番の証拠ではない。
この時点のホスト終了期限処理はビルド/Clippyまでの確認だった。その後のサンプル試験でbootstrap通常終了と保存済み再起動を確認した。
専用開発インスタンスの終了確認には、この実行環境でアクセスできるウィンドウがない制約がある。最新の片付け記録は [サンプルデバッグ記録](SAMPLE_DEBUG.md) を参照。

## サンプルプロジェクトを使った追加デバッグ

同日の追加試験で、隣接する `test.aup2` を元にコピーを作成し、AviUtl2 SDKによる実タイムライン描画を確認した。
元のファイルはSHA-256で変更なしを確認。SDKのバージョン/commitとCEFは基準から変更していない。

- 元プロジェクトの保存Webフォルダがnpmプロジェクトではなく、初期化に失敗した。CEF起動前のフォルダ/package.json/runtime検査と案内を追加。
- obj2のtext既定値にJSONの外側引用符が混入した。ホスト同梱Lua仕様に合わせて修正し、SDK値取得と画像で確認。
- 保存済みオブジェクトを含む起動で、初期化nonce 0が通知nonce 1で上書きされタイムアウトした。初期化にもACK保持を適用し、起動時register/purgeで解除しないよう修正。
- opt-in `runtime-debug` featureでSDK検証を実行。通常ビルドから除外し、releaseスクリプトでは有効化を拒否する。
- 1920×1080 / 30fps内の320×180検証素材で、7非連続/再要求、Freezeのディスク保存/読込、解除、コピー保存/別起動読込が合格。
- 初回と再起動後の5枚のPNG全体が一致。Web5件、Rust12件、fmt/Clippy、debug/release通常パッケージが成功。
- 検証workerの終了時再読込でホストEndDraw例外を再現し、再読込を除去した。修正後の再試験にその例外はない。

SDK保存はバックアップ形式であり、通常GUI保存/出力プラグインPNG連番を成功扱いしない。
bootstrapでは通常停止を確認。編集済みホストのWM_CLOSE後は外部から確認ウィンドウにアクセスできず、保存済みコピーを確認して専用プロセスだけを停止した。
証拠、再現コマンド、現在の制約は [SAMPLE_DEBUG.md](SAMPLE_DEBUG.md)。

## ロードマップの状態

| Phase | 状態 | 残る完了条件 |
|---|---|---|
| 0 | 基準再現・機能表作成済み | DPI測定を含む環境追加記録 |
| 1 | Windows debug/release、P0境界修正、配置スクリプト、SDK通常/Freeze画像を検証済み | 別のクリーンWindowsでの再現、GUI入力確認 |
| 2 | 起動/readiness/pump/初期化ACK/停止修正、CEF再初期化/正常停止/パス、ホストobj2/保存済み起動成功 | 明示的状態機械、強制終了からの自動復旧、設定メニュー/切替/編集後終了のGUI受入試験 |
| 3 | 要求ID/native ACK契約と実CEF画像試験を追加 | ready API、素材revision、CSS/WAAPI/SVG/p5混在、プレビューとPNG連番一致 |
| 4 | SDK Freeze/絶対パス保存復元の限定試験成功 | revisionキャッシュ、移動/相対パス、GUI保存復元 |
| 5–7 | 完了条件未着手 | 制作互換、色/性能/長時間、CI/クリーン導入 |

次はPhase 2の状態管理・障害復旧とAviUtl2 GUIスモークを進め、Phase 3の素材別画像試験へ進む。
この環境のビルド成功だけをクリーン環境や初期リリースの完成として扱わない。

## 2026-10-06: 指定p5/MIDI素材の試験

ユーザーが明示的に指定した `E:\Downloads\toybox\p5js` のMIDI Pattern Gridと既存テストを使用した。
指定素材を変更せず `.local-tests/p5js` にコピーし、付属ジェネレーターでSMF 0 / PPQ 480 / 24音 / 120→150 BPMのMIDIを生成。
元の型チェック/単体9件、Web5件、Rust12件が成功。p5画像の64行予約と非同期setupの一回実行/例外応答を修正した。
setup修正時に生じたプレビューPromise参照の問題も修正し、実SDKの準備待ちと9回の描画で確認した。

CEFとAviUtl2 SDKでセル配置と描画を確認したが、逆シーク後の画素差、2.8秒境界のstep遅れを再現した。
p5 2.2.1と2.3.4で画像差が起き、初期リリースのp5再現性条件は未達。結果を `issues_detected` と記録した。
SDKの生成スクリプト確認をGetGUIThreadInfoによる対象PID限定のRust診断で見つけ、専用ホストの登録/通常保存確認を操作して終了も確認した。
原本ファイルと既存MIDIはハッシュ一致、通常利用のホストは変更せず、テストコピーは配布物の依存に含めない。
[生成MIDI、PNG、ログ、残る不一致](P5_MIDI_TEST.md)。

## 2026-10-06: himawari_receiverの3object

ユーザー指定GitHubリポジトリをcommit `378ea63dd998bb579f8d3ca74c19f9560584a6e3` でcloneし、Drum/Kaiwai Phrase/Synth Soloとutilsを変更せず作業コピーで実行した。
RustでSMF 1 / PPQ 480 / 15 raw tracks / 184音 / 8小節 / 14.4秒のMIDIを生成し、参照されるトラックoffset、120→150 BPM、pitch bend、phrase fadeを検査した。
debug/release CEF各95要求が合格。初回Vite最適化による再読込/タイムアウトをテスト設定の事前最適化で解消し、未作成キャッシュからも95要求が合格した。
実AviUtl2 SDKでは3レイヤーに配置、16回の描画・逆シーク画像一致を確認し、専用ホストの通常保存確認を完了した。
別起動で保存済みコピーを読み込み、同じ16描画・逆シーク全RGBA一致と通常終了を確認した。
検証専用Rustのobject ID再利用と、通知streamを閉じる前のshutdownを修正した。元曲MIDIは使用せず、クローンの原本ハッシュを保持する。
この素材の結果は前回のMIDI Pattern Gridの不一致を解消したという主張ではない。[結果と再現手順](HIMAWARI_TEST.md)。

## 2026-10-06: CPU描画の性能改善

671ms級の描画を同じhimawari素材・debug版で再現し、OnPaint全4096×2304面の色変換と再コピーを除去した。
callback内で入力を借用し、要求範囲だけを従来と同じ丸めでstraight RGBAへ変換する。
debugの1画像中央値701.6/730.9/695.9msが30.2/32.3/37.3msとなった。
変更前後のCEF148枚・ホスト16枚・HTML5枚のPNGハッシュ一致、Rust14件、全target Clippyを確認した。
au2専用環境の3objectシーン取得は中央値115.1ms。通常停止を確認し、配布物と専用配置は検証featureなしの通常版へ戻した。
releaseには単発のIPC待ちが残り、Synth Soloのp95は改善していない。[計測条件、証拠、残る待ち](PERFORMANCE.md)。

## 2026-10-06: 単発遅延への追加対策

複数のアプローチを比較し、親PIDだけの監視をblocking workerへ退避、描画応答を通知待ちへ変更し、
64KiB以上で同色画素の多い応答を標準gRPC gzipで可逆圧縮した。Rust/CEF/aviutl2-rsのバージョンは維持する。
親監視だけ・ログ抑制だけでは1秒級の待ちが残ったが、可逆圧縮によって600描画の最大2999msが最終releaseで58ms/38msとなった。
最終debugも最大69msで、3回計1800連続描画の250ms超は0件。通常95要求を含め2085画像を全RGBA照合した。
CEF PNG222枚・SDK16枚・HTML5枚の変更前後一致、Rust16件、Clippy、au2の実ホストと通常終了を確認した。
3object合成のSDK取得中央値は124msで、前回の115msより速くはない。単体IPCの待ち改善と区別して記録する。
[候補・採否・再現手順・計測結果](LATENCY_APPROACHES.md)。
