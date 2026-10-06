# Windowsビルドと開発実行

Windows x64、Node 24、Rust stable MSVC、Visual Studio C++ツール、CMake、Ninjaを使用する。
固定SDKはaviutl2-rs 0.48.0 (fc94dbf9)、最小ホスト2.1.11。開発環境のAviUtl2 2.1.12で読込を確認済み。
CEFとcef-dll-sysは両方144.2.0+144.0.11に固定。ランタイムはCEF 144.0.11 / Chromium 144.0.7559.97。

## 取得と検証

```powershell
npm ci --prefix packages/vi5
npm ci --prefix examples/html
npm ci
npx playwright install chromium
npm run check:web
npm run build:web
npm test
cargo fmt --check
cargo check --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --locked
cargo build --release --locked
```

Cargo.lockとnpmのlockfileを使用する。初回はネットワーク接続・Cargo/npmキャッシュの書込が必要。
protocはbuild.rsがvendored実行ファイルを選択するため、別途PATHへ追加しなくてよい。
cef-dll-sysが対応CEFをダウンロードしCMake/Ninjaでwrapperを生成する。
CEF_PATHを指定する場合は、対応するarchive.jsonを含むcef-rsの展開ディレクトリを使う。
参考: [cef-rsのセットアップ](https://github.com/tauri-apps/cef-rs/blob/dev/README.md)。

## パッケージ

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1 -Profile debug
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-windows.ps1 -Profile release
```

署名を要求するPowerShell環境では上記のプロセス単位の指定を使う。システムの実行ポリシーを変更しない。
ビルド済みなら `-SkipBuild` を使える。
スクリプトはサーバーbuild.rsが出力する `target/<profile>/cef-runtime-path.txt` を読み、
archive.jsonのCEFバージョンと必須DLL/PAK/DAT/BIN/localesを確認する。
異なるバージョンや欠けたファイルはパッケージ作成前にエラーになる。

出力: `dist/windows-debug/Plugin/web-render`、`dist/windows-release/Plugin/web-render`。
現在は開発用の成果物であり、初期リリースの全受入試験完了を示すものではない。
WebプロジェクトはまだNode/npmとweb-renderのローカル依存を必要とする。

## au2による配置

```powershell
au2 prepare:aviutl2
# build-windows.ps1実行後に生成される、CEF全ファイルを含む定義を使う
au2 -C .aviutl2-runtime.toml dev --detach
```

通常のAviUtl2インストールは変更せず `.aviutl2-cli/development` に配置する。
au2 0.10.2のcopy方式はディレクトリを直接扱えず、symlink方式はWindows権限が必要だった。
そこでビルドスクリプトが全CEFファイル・localesを個別のcopy artifactとして生成する。
この生成ファイルは作業環境固有で、Gitには含めない。
au2のcmd.exeによるビルドで `\\?\` パスが作業ディレクトリに使われる問題には、
絶対パスのスクリプトをプロセス単位のEncodedCommandで実行して対応している。

プラグインのロード・メニュー・Luaモジュールは開発ログで確認する。
HTML設定、obj2の追加、Freeze、保存復元、PNG連番はGUI受入試験として別に記録する。
SDK経由のobj2・タイムライン画像・Freeze・保存済みサンプル再起動は [サンプルデバッグ手順](SAMPLE_DEBUG.md) で再現できる。
検証専用featureは `-Profile debug -RuntimeDebug` で有効化する。既定ビルドには含まず、releaseでの指定は拒否する。

## CEFから取得した画像の検証

```powershell
cargo run --locked -p web-render-cef --example verify_frames -- dist/windows-debug/Plugin/web-render/web-render-cef-server.exe examples/html docs/proofs
cargo run --locked -p web-render-cef --example verify_frames -- dist/windows-release/Plugin/web-render/web-render-cef-server.exe examples/html docs/proofs/release
```

15要求の順送り・逆送り・非連続要求・再要求、30000/1001fpsで実際のIPC RGBAを検査する。
全面色の画素、左右バーコード、同じ入力の全RGBA一致を確認しPNGと結果を保存する。
要求ID付きエラー、エラー後の描画、再初期化、10秒以内の正常終了も検査する。
EXE/CEF一式とnpm依存を導入済みのWebプロジェクトを空白・日本語のパスへ置いて同じコマンドを実行できる。
この環境ではそのパス試験も合格。環境・残る受入試験は [VALIDATION.md](VALIDATION.md) を参照。
これはAviUtl2のプレビュー/書き出しを経由した試験ではない。
