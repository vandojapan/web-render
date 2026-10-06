# AviUtl2サンプル

`html-sample.aup2` は1920×1080 / 30fps、150フレームで「フレーム検証」を配置したデバッグ済みコピーです。展開フォルダに隣接する元の `test.aup2` は変更していません。

Web依存を準備し、専用開発環境へプラグインを配置してから開きます。Webプロジェクトは `examples/html` です。初回はobj2生成後にホストを再起動してください。

このファイルのWebフォルダは作業環境の絶対パスで保存されています。展開先を移動した場合は設定メニューから `examples/html` を選択するか、空の元プロジェクトから `prepare_sample` で別名コピーを生成します。

SDKを使った検証手順、画像、制約は [サンプルデバッグ記録](../../docs/SAMPLE_DEBUG.md) にあります。

`p5-midi-sample.aup2` は、今回ユーザーが指定したMIDI Pattern Gridを配置したテスト用コピーです。
Web素材は `.local-tests/p5js` なので、`scripts/prepare-p5-midi-test.ps1 -Source <指定p5jsフォルダ>` でコピーと依存を準備してから開きます。
逆シーク画像差と時刻境界の問題が残るため、完全一致の検証済みサンプルではありません。[結果](../../docs/P5_MIDI_TEST.md)。

`himawari-sample.aup2` は、指定GitHubリポジトリのDrum/Kaiwai Phrase/Synth Soloを配置した16秒のコピーです。生成MIDIとWeb素材は `.local-tests/himawari-receiver` にあります。cloneと `scripts/prepare-himawari-test.ps1` の実行が必要です。[MIDIと検証結果](../../docs/HIMAWARI_TEST.md)。この試験の逆シーク画像は一致しています。
