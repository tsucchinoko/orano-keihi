<script lang="ts">
// スタイルテストページ
let inputValue = $state("");
let selectValue = $state("");
</script>

<div class="test-page">
	<h1 class="page-title">CSSスタイル検証ページ</h1>
	<p class="page-subtitle">リファクタリング後のスタイルを視覚的に確認します</p>

	<!-- カードコンポーネントのテスト -->
	<section class="test-section">
		<h2 class="section-title">1. カードコンポーネント</h2>
		<div class="test-grid">
			<div class="card">
				<h3>基本カード</h3>
				<p>これは基本的なカードコンポーネントです。ホバーすると影が濃くなります。</p>
			</div>
			<div class="card bg-blue-500 p-8">
				<h3 class="text-white">Tailwindで上書き</h3>
				<p class="text-white">bg-blue-500とp-8で上書きしています。</p>
			</div>
		</div>
	</section>

	<!-- ボタンコンポーネントのテスト -->
	<section class="test-section">
		<h2 class="section-title">2. ボタンコンポーネント</h2>
		<div class="test-grid">
			<button type="button" class="btn btn-primary">プライマリボタン</button>
			<button type="button" class="btn btn-success">サクセスボタン</button>
			<button type="button" class="btn btn-info">インフォボタン</button>
			<button type="button" class="btn btn-warning">ワーニングボタン</button>
			<button type="button" class="btn btn-primary text-lg px-8">
				Tailwindで上書き
			</button>
		</div>
	</section>

	<!-- 入力フィールドのテスト -->
	<section class="test-section">
		<h2 class="section-title">3. 入力フィールド</h2>
		<div class="test-grid">
			<div>
				<label for="test-input" class="block mb-2 font-semibold">基本入力</label>
				<input
					id="test-input"
					type="text"
					class="input"
					placeholder="テキストを入力してください"
					bind:value={inputValue}
				/>
			</div>
			<div>
				<label for="test-select" class="block mb-2 font-semibold">セレクト</label>
				<select id="test-select" class="input" bind:value={selectValue}>
					<option value="">選択してください</option>
					<option value="1">オプション1</option>
					<option value="2">オプション2</option>
				</select>
			</div>
			<div>
				<label for="test-textarea" class="block mb-2 font-semibold">テキストエリア</label>
				<textarea
					id="test-textarea"
					class="input min-h-24"
					placeholder="複数行のテキストを入力してください"
				></textarea>
			</div>
			<div>
				<label for="test-input-override" class="block mb-2 font-semibold">
					Tailwindで上書き
				</label>
				<input
					id="test-input-override"
					type="text"
					class="input bg-yellow-100 border-yellow-500"
					placeholder="背景色とボーダーを上書き"
				/>
			</div>
		</div>
	</section>

	<!-- カテゴリカラーのテスト -->
	<section class="test-section">
		<h2 class="section-title">4. カテゴリカラー</h2>
		<div class="test-grid">
			<div class="card">
				<div class="bg-category-transport text-white p-4 rounded">交通費</div>
			</div>
			<div class="card">
				<div class="bg-category-meals text-white p-4 rounded">飲食費</div>
			</div>
			<div class="card">
				<div class="bg-category-communication text-white p-4 rounded">通信費</div>
			</div>
			<div class="card">
				<div class="bg-category-supplies text-white p-4 rounded">消耗品費</div>
			</div>
			<div class="card">
				<div class="bg-category-entertainment text-white p-4 rounded">接待交際費</div>
			</div>
			<div class="card">
				<div class="bg-category-other text-white p-4 rounded">その他</div>
			</div>
		</div>
	</section>

	<!-- グラデーションのテスト -->
	<section class="test-section">
		<h2 class="section-title">5. グラデーション</h2>
		<div class="test-grid">
			<div class="gradient-primary text-white p-8 rounded-xl">
				<h3 class="font-bold">プライマリグラデーション</h3>
				<p>gradient-primaryクラス</p>
			</div>
			<div class="gradient-success text-white p-8 rounded-xl">
				<h3 class="font-bold">サクセスグラデーション</h3>
				<p>gradient-successクラス</p>
			</div>
			<div class="gradient-info text-white p-8 rounded-xl">
				<h3 class="font-bold">インフォグラデーション</h3>
				<p>gradient-infoクラス</p>
			</div>
			<div class="gradient-warning text-white p-8 rounded-xl">
				<h3 class="font-bold">ワーニンググラデーション</h3>
				<p>gradient-warningクラス</p>
			</div>
		</div>
	</section>

	<!-- インタラクション状態のテスト -->
	<section class="test-section">
		<h2 class="section-title">6. インタラクション状態</h2>
		<div class="test-grid">
			<div class="card">
				<h3 class="font-bold mb-2">ホバー状態</h3>
				<p class="text-sm text-gray-600 mb-4">
					カードにマウスを乗せると影が濃くなります
				</p>
				<button type="button" class="btn btn-primary">
					ボタンにホバーすると浮き上がります
				</button>
			</div>
			<div class="card">
				<h3 class="font-bold mb-2">フォーカス状態</h3>
				<p class="text-sm text-gray-600 mb-4">
					入力フィールドをクリックするとリングが表示されます
				</p>
				<input
					type="text"
					class="input"
					placeholder="クリックしてフォーカス"
				/>
			</div>
			<div class="card">
				<h3 class="font-bold mb-2">アクティブ状態</h3>
				<p class="text-sm text-gray-600 mb-4">
					ボタンをクリックすると元の位置に戻ります
				</p>
				<button type="button" class="btn btn-info">
					クリックしてアクティブ状態を確認
				</button>
			</div>
		</div>
	</section>

	<!-- レスポンシブテスト -->
	<section class="test-section">
		<h2 class="section-title">7. レスポンシブレイアウト</h2>
		<div class="responsive-test">
			<div class="card">
				<h3 class="font-bold mb-2">レスポンシブカード</h3>
				<p class="text-sm text-gray-600">
					画面サイズを変更して、レイアウトが適切に調整されることを確認してください。
					モバイル、タブレット、デスクトップの各サイズで表示を確認します。
				</p>
			</div>
		</div>
	</section>
</div>

<style>
	.test-page {
		max-width: 1200px;
		margin: 0 auto;
		padding: 2rem;
	}

	.page-title {
		font-size: 2.5rem;
		font-weight: 700;
		background: var(--gradient-primary);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin-bottom: 0.5rem;
		text-align: center;
	}

	.page-subtitle {
		color: #6b7280;
		font-size: 1.125rem;
		text-align: center;
		margin-bottom: 3rem;
	}

	.test-section {
		margin-bottom: 3rem;
	}

	.section-title {
		font-size: 1.5rem;
		font-weight: 700;
		color: #1f2937;
		margin-bottom: 1.5rem;
		padding-bottom: 0.5rem;
		border-bottom: 2px solid #e5e7eb;
	}

	.test-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 1.5rem;
	}

	.responsive-test {
		display: grid;
		grid-template-columns: 1fr;
		gap: 1.5rem;
	}

	/* レスポンシブデザイン */
	@media (max-width: 768px) {
		.test-page {
			padding: 1rem;
		}

		.page-title {
			font-size: 2rem;
		}

		.test-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
