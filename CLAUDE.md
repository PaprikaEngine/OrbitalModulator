# モジュールシンセサイザー開発仕様書

## プロジェクト概要
モジュラーシンセサイザーのCUIアプリケーション開発

## 技術スタック
- **コア**: Rust
- **オーディオエンジン**: CPAL + FunDSP + DASP
- **インターフェース**: コマンドライン

## 基本コンセプト
- **ノードベースアーキテクチャ**: 各モジュールはノードとして実装
- **ノード間接続**: ノード同士を接続してシグナルルーティングを構築
- **グラフ処理**: ノードグラフを構築し、信号フローを管理
- **CUI操作**: コマンドラインでノード作成・接続・制御

## 一般的なモジュラーシンセサイザーの基本構成

### 音声信号チェーン（Audio Signal Path）
**VCO → VCF → VCA** の基本的な信号チェーン

1. **VCO (Voltage Controlled Oscillator)** - 電圧制御オシレーター
   - 基本的な音波を生成
   - 周波数（ピッチ）を制御
   - 波形選択（サイン波、三角波、ノコギリ波、パルス波等）

2. **VCF (Voltage Controlled Filter)** - 電圧制御フィルター
   - 音色（ティンバー）を制御
   - ローパス、ハイパス、バンドパス、ノッチフィルター
   - カットオフ周波数とレゾナンス制御

3. **VCA (Voltage Controlled Amplifier)** - 電圧制御アンプ
   - 音量（アンプリチュード）を制御
   - エンベロープによる動的な音量変化
   - 最終出力段階

### 制御信号源（Control Sources）
4. **Envelope Generator (EG/ADSR)** - エンベロープジェネレーター
   - Attack, Decay, Sustain, Release
   - VCAやVCFの時間的変化を制御

5. **LFO (Low Frequency Oscillator)** - 低周波オシレーター  
   - モジュレーション用の低周波信号
   - トレモロ、ビブラート効果

### 制御電圧仕様
- **1V/Oct**: 1ボルトで1オクターブの標準
- **Gate信号**: 鍵盤のオン/オフ
- **Trigger信号**: エンベロープの開始

### 追加モジュール
- **Mixer** - 複数信号のミックス
- **Multiple** - 信号分配
- **Attenuator** - 信号減衰
- **Sample & Hold** - ランダム電圧生成
- **Ring Modulator** - 周波数変調
- **Sequencer** - パターンシーケンス
- **Noise Generator** - ノイズ生成

## プラグインシステム仕様

### サードパーティモジュール対応
**目的:**
- 開発者がカスタムモジュールを追加可能
- コミュニティによる機能拡張
- オープンエコシステムの構築

### プラグインアーキテクチャ
**技術スタック:**
- **Rust側**: プラグインローダー、統一API
- **Tauri**: プラグイン管理UI、インストール機能
- **JavaScript/React**: プラグインUI統合

**プラグイン形式:**
- **Rust DLL/dylib**: オーディオ処理部分
- **JavaScript module**: UI部分（React Component）
- **JSON manifest**: モジュール定義、メタデータ

### プラグインAPI仕様
**基本インターフェース:**
```rust
trait SynthModule {
    fn process_audio(&mut self, inputs: &[AudioBuffer], outputs: &mut [AudioBuffer]);
    fn process_cv(&mut self, cv_inputs: &[f32]) -> Vec<f32>;
    fn set_parameter(&mut self, param_id: u32, value: f32);
    fn get_parameter(&self, param_id: u32) -> f32;
    fn get_module_info() -> ModuleInfo;
}
```

**モジュール定義:**
```json
{
  "name": "CustomFilter",
  "version": "1.0.0",
  "author": "Developer Name",
  "description": "カスタムフィルターモジュール",
  "category": "Filter",
  "inputs": [
    {"id": "audio_in", "type": "Audio Stereo"},
    {"id": "cutoff_cv", "type": "CV"}
  ],
  "outputs": [
    {"id": "audio_out", "type": "Audio Stereo"}
  ],
  "parameters": [
    {"id": "cutoff", "name": "Cutoff", "min": 20, "max": 20000, "default": 1000}
  ],
  "ui_component": "CustomFilterUI"
}
```

### プラグイン管理機能
**プラグインストア:**
- プラグインブラウジング
- レーティング・レビュー機能
- 自動アップデート
- 依存関係管理

**インストール機能:**
- ワンクリックインストール
- プラグイン検証・サンドボックス
- アンインストール機能
- プラグイン設定管理

**開発者サポート:**
- プラグインSDK提供
- ドキュメント・サンプル
- デバッグツール
- プラグイン投稿システム

### セキュリティ・安定性
**サンドボックス化:**
- プラグインの分離実行
- システムリソースへの制限アクセス
- クラッシュ時の保護

**署名・検証:**
- 開発者署名
- プラグインハッシュ検証
- 公式認証システム

**パフォーマンス管理:**
- CPU使用率制限
- メモリ使用量監視
- リアルタイム処理保証

## 決定事項

### オシレーターノード（Generator Node）
**ノードタイプ:** `oscillator`

**波形タイプ:**

- 三角波 (Triangle Wave)
- ノコギリ波 (Sawtooth Wave)  
- サイン波 (Sine Wave)
- パルス波 (Pulse Wave)

**パラメーター:**

- 周波数 (Hz) - 20Hz ~ 20kHz
- 波形選択 - コマンドで指定
- 音量 (Amplitude) - 0.0 ~ 1.0

**入力ポート:**

- frequency_cv (CV入力) - 周波数制御
- amplitude_cv (CV入力) - 音量制御

**出力ポート:**

- audio_out (Audio Mono) - オーディオ出力

### ノード間接続仕様

**ポートタイプ:**

1. **Audio Mono** - モノラルオーディオ信号（-1.0 ~ 1.0の浮動小数点配列）
2. **Audio Stereo** - ステレオオーディオ信号（L/Rチャンネルのペア）
3. **CV (Control Voltage)** - 制御信号（-10.0V ~ 10.0Vの浮動小数点値）

**データ形式:**

- サンプリングレート: 44.1kHz（設定可能）
- バッファサイズ: 512サンプル（調整可能）
- ビット深度: 32bit float

**接続ルール:**

- 同じタイプのポート間のみ接続可能（AudioMono ↔ AudioMono, CV ↔ CV等）
- 1つの入力ポートには1つの出力ポートのみ接続可能
- 1つの出力ポートから複数の入力ポートへの分岐は可能
- ノード自身への接続（自己ループ）は禁止
- 循環参照（A→B→A）の接続は自動検出して禁止

**循環参照防止機能:**

1. **事前検証**: 接続追加時に循環が発生しないかチェック
2. **グラフ検証**: `validate_graph()`メソッドで全体の循環チェック
3. **トポロジカルソート**: 処理順序決定時に循環を検出
4. **エラー処理**: 循環検出時は接続を拒否し、明確なエラーメッセージを表示

**ノードグラフ処理:**

- DAG（有向非循環グラフ）として構築
- トポロジカルソートによる処理順序決定
- リアルタイム循環検出とサイクル回避

**接続コマンド:**

- `connect <source_node>:<source_port> <target_node>:<target_port>` - 名前ベース接続
- `connect-by-id <source_id> <source_port> <target_id> <target_port>` - IDベース接続
- `disconnect <source_node>:<source_port> <target_node>:<target_port>` - 名前ベース切断
- `disconnect-by-id <source_id> <source_port> <target_id> <target_port>` - IDベース切断

### Audio Outputノード
**ノードタイプ:** `output`

**機能:**

- 最終的な音声出力（スピーカー/ヘッドフォン）
- システムのオーディオデバイスへの出力

**パラメーター:**

- master_volume (Master Volume) - 0.0 ~ 1.0
- mute (Mute) - true/false

**入力ポート:**

- audio_in_l (Audio Mono) - 左チャンネル入力
- audio_in_r (Audio Mono) - 右チャンネル入力
- master_volume_cv (CV入力) - マスター音量制御

**出力ポート:**

- なし（最終出力ノード）

## CUIコマンド仕様

**基本コマンド構造:**

```bash
orbital-modulator [COMMAND] [OPTIONS]
```

**主要コマンド:**

- `create [NODE_TYPE] [NAME]` - ノード作成
- `connect [SOURCE_NODE:PORT] [TARGET_NODE:PORT]` - ノード間接続
- `disconnect [SOURCE_NODE:PORT] [TARGET_NODE:PORT]` - ノード間切断
- `set [NODE] [PARAM] [VALUE]` - ノードパラメーター設定
- `get [NODE] [PARAM]` - ノードパラメーター取得
- `play` - 再生開始
- `stop` - 再生停止
- `list` - ノード一覧表示
- `info [NODE]` - ノード詳細情報表示
- `graph` - ノードグラフ表示
- `save [FILENAME]` - 設定保存
- `load [FILENAME]` - 設定読み込み

**使用例:**

```bash
# オシレーターノード作成
orbital-modulator create oscillator osc1
orbital-modulator set osc1 frequency 440
orbital-modulator set osc1 waveform sine

# アウトプットノード作成
orbital-modulator create output main_out

# ノード間接続（ポート指定）
orbital-modulator connect osc1:audio_out main_out:audio_in_l
orbital-modulator connect osc1:audio_out main_out:audio_in_r

# ノードグラフ確認
orbital-modulator graph

# 再生
orbital-modulator play
```

## アーキテクチャ設計

**ノード管理システム:**

- ノードID管理（UUID）
- ノードタイプレジストリ
- パラメーター管理
- ポート管理（入力・出力）

**グラフエンジン:**

- ノードグラフ構築・管理
- 接続管理（エッジ）
- トポロジカルソート処理
- サイクル検出・回避

**オーディオエンジン:**

- CPAL: クロスプラットフォームオーディオI/O
- FunDSP: 高性能DSP処理
- DASP: 基本デジタル信号処理
- リアルタイムオーディオコールバック

**コマンド処理:**

- CLIパーサー（clap使用）
- コマンド実行エンジン
- 設定永続化（JSON/TOML）

**コアデータ構造:**

```rust
pub struct Node {
    id: Uuid,
    node_type: String,
    parameters: HashMap<String, f32>,
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
}

pub struct Port {
    name: String,
    port_type: PortType,
    connections: Vec<Connection>,
}

pub struct Connection {
    source_node: Uuid,
    source_port: String,
    target_node: Uuid,
    target_port: String,
}

pub struct AudioGraph {
    nodes: HashMap<Uuid, Node>,
    connections: Vec<Connection>,
    processing_order: Vec<Uuid>,
}
```

## 次のステップ

- [ ] 基本プロジェクト構造作成
- [ ] ノード・グラフシステム実装
- [ ] CLIフレームワーク実装
- [ ] 基本オシレーターノード実装
- [ ] Audio Outputノード実装
- [ ] ノード接続システム実装
- [ ] オーディオエンジン統合
- [ ] 設定保存・読み込み機能実装