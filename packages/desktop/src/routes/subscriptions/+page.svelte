<script lang="ts">
import { SubscriptionForm, SubscriptionList } from "$features/subscriptions";
import type { Subscription } from "$lib/types";

// モーダル表示状態
let showForm = $state(false);
let editingSubscription = $state<Subscription | undefined>(undefined);

// フォーム表示
function handleAddNew() {
	editingSubscription = undefined;
	showForm = true;
}

// 編集
function handleEdit(subscription: Subscription) {
	editingSubscription = subscription;
	showForm = true;
}

// フォーム成功時
function handleFormSuccess() {
	showForm = false;
	editingSubscription = undefined;
}

// フォームキャンセル時
function handleFormCancel() {
	showForm = false;
	editingSubscription = undefined;
}
</script>

<div class="subscriptions-page">
	<!-- ヘッダー -->
	<div class="page-header">
		<div>
			<h1 class="page-title">サブスクリプション管理</h1>
			<p class="page-subtitle">月額・年額のサブスクリプションを管理</p>
		</div>
		<button
			type="button"
			onclick={handleAddNew}
			class="btn btn-primary"
		>
			➕ 新規追加
		</button>
	</div>

	<!-- サブスクリプション一覧 -->
	<div class="content-section">
		<SubscriptionList onEdit={handleEdit} />
	</div>

	<!-- フォームモーダル -->
	{#if showForm}
		<div class="modal-overlay" onclick={handleFormCancel}>
			<div class="modal-content" onclick={(e) => e.stopPropagation()}>
				<SubscriptionForm
					subscription={editingSubscription}
					onSuccess={handleFormSuccess}
					onCancel={handleFormCancel}
				/>
			</div>
		</div>
	{/if}
</div>

<style>
	.subscriptions-page {
		display: flex;
		flex-direction: column;
		gap: 2rem;
	}

	/* ページヘッダー */
	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 2rem;
	}

	.page-title {
		font-size: 2.5rem;
		font-weight: 700;
		background: var(--gradient-primary);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin: 0;
	}

	.page-subtitle {
		color: #6b7280;
		font-size: 1.125rem;
		margin-top: 0.5rem;
	}

	/* コンテンツセクション */
	.content-section {
		margin-top: 1rem;
	}

	/* モーダル */
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
		backdrop-filter: blur(4px);
	}

	.modal-content {
		background: white;
		border-radius: 16px;
		padding: 2rem;
		max-width: 600px;
		width: 100%;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
		animation: modalSlideIn 0.3s ease-out;
	}

	@keyframes modalSlideIn {
		from {
			opacity: 0;
			transform: translateY(-20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	/* レスポンシブデザイン */
	@media (max-width: 768px) {
		.page-header {
			flex-direction: column;
			align-items: flex-start;
		}

		.page-title {
			font-size: 2rem;
		}

		.modal-content {
			padding: 1.5rem;
		}
	}
</style>
