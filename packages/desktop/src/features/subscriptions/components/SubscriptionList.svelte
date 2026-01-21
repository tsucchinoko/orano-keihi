<script lang="ts">
import type { Subscription } from "$lib/types";
import { expenseStore } from "$lib/stores/expenses.svelte";
import { toastStore } from "$lib/stores/toast.svelte";
import { ReceiptViewer } from "$features/receipts";

// Props
interface Props {
	onEdit: (subscription: Subscription) => void;
}

let { onEdit }: Props = $props();

// 領収書表示状態
let showReceiptViewer = $state(false);
let currentReceiptUrl = $state<string | undefined>(undefined);
let currentReceiptPath = $state<string | undefined>(undefined);

// ストアからサブスクリプションデータを取得
const subscriptions = $derived(expenseStore.subscriptions);

// コンポーネントマウント時にサブスクリプションを読み込む
$effect(() => {
	expenseStore.loadSubscriptions();
	expenseStore.loadMonthlySubscriptionTotal();
});

// アクティブなサブスクリプション
const activeSubscriptions = $derived.by(() => {
	return subscriptions.filter((sub) => sub.is_active);
});

// 非アクティブなサブスクリプション
const inactiveSubscriptions = $derived.by(() => {
	return subscriptions.filter((sub) => !sub.is_active);
});

// 月額合計（ストアから取得）
const monthlyTotal = $derived(expenseStore.monthlySubscriptionTotal);

// ステータス切り替え処理
async function handleToggleStatus(id: number): Promise<void> {
	const success = await expenseStore.toggleSubscription(id);
	if (success) {
		const subscription = subscriptions.find((sub) => sub.id === id);
		if (subscription) {
			toastStore.success(
				subscription.is_active
					? "サブスクリプションを停止しました"
					: "サブスクリプションを再開しました",
			);
		}
	} else {
		toastStore.error(
			expenseStore.error || "ステータスの切り替えに失敗しました",
		);
	}
}

// 削除処理
async function handleDelete(id: number): Promise<void> {
	console.info(`🎯 UI: 削除ボタンクリック: subscription_id=${id}`);

	const success = await expenseStore.removeSubscription(id);
	console.info(`🎯 UI: 削除処理結果: success=${success}`);

	if (success) {
		toastStore.success("サブスクリプションを削除しました");
	} else {
		console.error(`🎯 UI: 削除失敗エラー:`, expenseStore.error);
		toastStore.error(expenseStore.error || "サブスクリプションの削除に失敗しました");
	}
}

// カテゴリアイコン
const categoryIcons: Record<string, string> = {
	交通費: "🚗",
	飲食費: "🍽️",
	通信費: "📱",
	消耗品費: "📦",
	接待交際費: "🤝",
	その他: "📋",
};

// カテゴリカラー
const categoryColors: Record<string, string> = {
	交通費: "bg-category-transport",
	飲食費: "bg-category-meals",
	通信費: "bg-category-communication",
	消耗品費: "bg-category-supplies",
	接待交際費: "bg-category-entertainment",
	その他: "bg-category-other",
};

// 金額フォーマット
function formatAmount(amount: number): string {
	return new Intl.NumberFormat("ja-JP", {
		style: "currency",
		currency: "JPY",
	}).format(amount);
}

// 月額換算
function getMonthlyAmount(subscription: Subscription): number {
	return subscription.billing_cycle === "annual"
		? subscription.amount / 12
		: subscription.amount;
}

// 次回支払日計算
function getNextBillingDate(subscription: Subscription): string {
	const startDate = new Date(subscription.start_date);
	const today = new Date();

	if (subscription.billing_cycle === "monthly") {
		// 月払い：次の月の同じ日
		const nextDate = new Date(
			today.getFullYear(),
			today.getMonth() + 1,
			startDate.getDate(),
		);
		return nextDate.toLocaleDateString("ja-JP", {
			year: "numeric",
			month: "long",
			day: "numeric",
		});
	} else {
		// 年払い：次の年の同じ日
		const nextDate = new Date(
			today.getFullYear() + 1,
			startDate.getMonth(),
			startDate.getDate(),
		);
		return nextDate.toLocaleDateString("ja-JP", {
			year: "numeric",
			month: "long",
			day: "numeric",
		});
	}
}

// 領収書表示ハンドラー
function handleViewReceipt(receiptPath?: string) {
	if (!receiptPath) return;

	currentReceiptUrl = receiptPath.startsWith('http') ? receiptPath : undefined;
	currentReceiptPath = !receiptPath.startsWith('http') ? receiptPath : undefined;
	showReceiptViewer = true;
}

// 領収書表示を閉じる
function handleCloseReceiptViewer() {
	showReceiptViewer = false;
	currentReceiptUrl = undefined;
	currentReceiptPath = undefined;
}
</script>

<div class="space-y-6">
	<!-- 月額合計カード -->
	<div class="card bg-gradient-to-br from-purple-50 to-pink-50">
		<h3 class="text-lg font-bold mb-2">月額合計</h3>
		<div class="text-3xl font-bold bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
			{formatAmount(monthlyTotal)}
		</div>
		<p class="text-sm text-gray-600 mt-2">
			アクティブなサブスクリプション: {activeSubscriptions.length}件
		</p>
	</div>

	<!-- アクティブなサブスクリプション -->
	{#if activeSubscriptions.length > 0}
		<div>
			<h3 class="text-xl font-bold mb-4 flex items-center gap-2">
				<span class="w-3 h-3 bg-green-500 rounded-full"></span>
				アクティブ
			</h3>
			<div class="space-y-3">
				{#each activeSubscriptions as subscription (subscription.id)}
					<div class="card hover:shadow-lg transition-all duration-200 relative overflow-hidden">
						<!-- カテゴリカラーバー -->
						<div class="absolute top-0 left-0 w-1 h-full {categoryColors[subscription.category] || 'bg-category-other'}"></div>

						<div class="pl-4">
							<div class="flex items-start justify-between gap-4">
								<!-- 左側：サブスクリプション情報 -->
								<div class="flex-1">
									<div class="flex items-center gap-2 mb-2">
										<span class="text-2xl">{categoryIcons[subscription.category] || '📋'}</span>
										<h4 class="text-lg font-bold">{subscription.name}</h4>
									</div>

									<div class="flex items-baseline gap-2 mb-2">
										<span class="text-2xl font-bold bg-gradient-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent">
											{formatAmount(subscription.amount)}
										</span>
										<span class="text-sm text-gray-500">
											/ {subscription.billing_cycle === 'monthly' ? '月' : '年'}
										</span>
									</div>

									{#if subscription.billing_cycle === 'annual'}
										<p class="text-sm text-gray-600 mb-2">
											月額換算: {formatAmount(getMonthlyAmount(subscription))}
										</p>
									{/if}

									<div class="flex items-center gap-4 text-sm text-gray-600">
										<span>{categoryIcons[subscription.category]} {subscription.category}</span>
										<span>📅 次回: {getNextBillingDate(subscription)}</span>
									</div>

									<!-- 領収書表示ボタン -->
									{#if subscription.receipt_path}
										<div class="mt-2">
											<button
												type="button"
												onclick={() => handleViewReceipt(subscription.receipt_path)}
												class="inline-flex items-center gap-2 text-sm text-blue-600 hover:text-blue-800 transition-colors"
											>
												📄 領収書を表示
											</button>
										</div>
									{/if}
								</div>

								<!-- 右側：アクションボタン -->
								<div class="flex flex-col gap-2">
									<button
										type="button"
										onclick={() => onEdit(subscription)}
										class="btn btn-info text-sm px-3 py-1"
										title="編集"
									>
										✏️ 編集
									</button>
									<button
										type="button"
										onclick={() => handleToggleStatus(subscription.id)}
										class="btn bg-gray-500 hover:bg-gray-600 text-white text-sm px-3 py-1"
										title="無効化"
									>
										⏸️ 停止
									</button>
									<button
										type="button"
										onclick={() => handleDelete(subscription.id)}
										class="btn btn-danger text-sm px-3 py-1"
										title="削除"
									>
										🗑️ 削除
									</button>
								</div>
							</div>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<!-- 非アクティブなサブスクリプション -->
	{#if inactiveSubscriptions.length > 0}
		<div>
			<h3 class="text-xl font-bold mb-4 flex items-center gap-2">
				<span class="w-3 h-3 bg-gray-400 rounded-full"></span>
				停止中
			</h3>
			<div class="space-y-3">
				{#each inactiveSubscriptions as subscription (subscription.id)}
					<div class="card opacity-60 hover:opacity-100 hover:shadow-lg transition-all duration-200 relative overflow-hidden">
						<!-- カテゴリカラーバー -->
						<div class="absolute top-0 left-0 w-1 h-full {categoryColors[subscription.category] || 'bg-category-other'}"></div>

						<div class="pl-4">
							<div class="flex items-start justify-between gap-4">
								<!-- 左側：サブスクリプション情報 -->
								<div class="flex-1">
									<div class="flex items-center gap-2 mb-2">
										<span class="text-2xl grayscale">{categoryIcons[subscription.category] || '📋'}</span>
										<h4 class="text-lg font-bold text-gray-600">{subscription.name}</h4>
									</div>

									<div class="flex items-baseline gap-2 mb-2">
										<span class="text-2xl font-bold text-gray-500">
											{formatAmount(subscription.amount)}
										</span>
										<span class="text-sm text-gray-400">
											/ {subscription.billing_cycle === 'monthly' ? '月' : '年'}
										</span>
									</div>

									<div class="text-sm text-gray-500">
										{categoryIcons[subscription.category]} {subscription.category}
									</div>

									<!-- 領収書表示ボタン -->
									{#if subscription.receipt_path}
										<div class="mt-2">
											<button
												type="button"
												onclick={() => handleViewReceipt(subscription.receipt_path)}
												class="inline-flex items-center gap-2 text-sm text-blue-600 hover:text-blue-800 transition-colors"
											>
												📄 領収書を表示
											</button>
										</div>
									{/if}
								</div>

								<!-- 右側：アクションボタン -->
								<div class="flex flex-col gap-2">
									<button
										type="button"
										onclick={() => onEdit(subscription)}
										class="btn btn-info text-sm px-3 py-1"
										title="編集"
									>
										✏️ 編集
									</button>
									<button
										type="button"
										onclick={() => handleToggleStatus(subscription.id)}
										class="btn btn-success text-sm px-3 py-1"
										title="有効化"
									>
										▶️ 再開
									</button>
									<button
										type="button"
										onclick={() => handleDelete(subscription.id)}
										class="btn btn-danger text-sm px-3 py-1"
										title="削除"
									>
										🗑️ 削除
									</button>
								</div>
							</div>
						</div>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	<!-- データがない場合 -->
	{#if subscriptions.length === 0}
		<div class="card text-center py-12">
			<div class="text-6xl mb-4">💳</div>
			<p class="text-gray-500 text-lg">サブスクリプションがありません</p>
			<p class="text-gray-400 text-sm mt-2">新しいサブスクリプションを追加してください</p>
		</div>
	{/if}
</div>

<!-- 領収書表示モーダル -->
{#if showReceiptViewer && (currentReceiptUrl || currentReceiptPath)}
	<ReceiptViewer
		receiptUrl={currentReceiptUrl}
		receiptPath={currentReceiptPath}
		onClose={handleCloseReceiptViewer}
	/>
{/if}

<style>
	/* グラデーションホバー効果 */
	.card:hover::before {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: linear-gradient(135deg, rgba(102, 126, 234, 0.05) 0%, rgba(118, 75, 162, 0.05) 100%);
		pointer-events: none;
	}

	/* スムーズアニメーション */
	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(-10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.space-y-3 > div {
		animation: fadeIn 0.3s ease-out;
	}
</style>
