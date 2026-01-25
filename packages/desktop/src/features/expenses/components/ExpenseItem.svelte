<script lang="ts">
    import type { Expense } from "$lib/types";
    import { convertFileSrc } from "@tauri-apps/api/core";
    import { categoryStore } from "$lib/stores/categories.svelte";

    // Props
    interface Props {
        expense: Expense;
        onEdit: (expense: Expense) => void;
        onDelete: (id: number) => void;
        onViewReceipt?: (receiptUrl?: string, receiptPath?: string) => void;
    }

    let { expense, onEdit, onDelete, onViewReceipt }: Props = $props();

    // カテゴリーストアの初期化
    $effect(() => {
        categoryStore.initialize();
    });

    // 領収書のサムネイルURL（ローカルファイル用）
    let receiptThumbnailUrl = $state<string | undefined>(undefined);

    // 領収書パスを変換（後方互換性のため）
    $effect(() => {
        if (expense.receipt_path) {
            receiptThumbnailUrl = convertFileSrc(expense.receipt_path);
        } else {
            receiptThumbnailUrl = undefined;
        }
    });

    // 領収書が存在するかチェック
    const hasReceipt = $derived(() => {
        return !!(expense.receipt_url || expense.receipt_path);
    });

    // 領収書の種類を判定
    const isReceiptImage = $derived(() => {
        if (expense.receipt_url) {
            return /\.(png|jpg|jpeg)/i.test(expense.receipt_url);
        }
        if (expense.receipt_path) {
            return /\.(png|jpg|jpeg)$/i.test(expense.receipt_path);
        }
        return false;
    });

    // 削除確認ダイアログの状態
    let showDeleteConfirm = $state(false);

    // DBから取得したカテゴリー情報を使用
    // category_idが存在する場合はそれを優先、なければcategory名で検索（後方互換性）
    const categoryIcon = $derived(() => {
        if (expense.category_id) {
            return categoryStore.getIconById(expense.category_id);
        }
        return categoryStore.getIconByName(expense.category);
    });

    const categoryColorClass = $derived(() => {
        if (expense.category_id) {
            return categoryStore.getColorById(expense.category_id);
        }
        return categoryStore.getColorByName(expense.category);
    });

    // 日付フォーマット
    function formatDate(dateStr: string): string {
        const date = new Date(dateStr);
        return date.toLocaleDateString("ja-JP", {
            year: "numeric",
            month: "long",
            day: "numeric",
        });
    }

    // 金額フォーマット
    function formatAmount(amount: number): string {
        return new Intl.NumberFormat("ja-JP", {
            style: "currency",
            currency: "JPY",
        }).format(amount);
    }

    // 削除確認
    function confirmDelete() {
        console.info(`⚠️ 削除確認ダイアログ表示: expense_id=${expense.id}`);
        showDeleteConfirm = true;
    }

    // 削除実行
    function handleDelete() {
        console.info(`🔴 削除ボタンクリック: expense_id=${expense.id}`);
        onDelete(expense.id);
        showDeleteConfirm = false;
    }

    // 削除キャンセル
    function cancelDelete() {
        showDeleteConfirm = false;
    }

    // 領収書表示
    function handleViewReceipt() {
        if (onViewReceipt) {
            onViewReceipt(expense.receipt_url, expense.receipt_path);
        }
    }
</script>

<div
    class="card hover:shadow-lg transition-shadow duration-200 relative overflow-hidden"
>
    <!-- カテゴリカラーバー -->
    <div class="absolute top-0 left-0 w-1 h-full {categoryColorClass()}"></div>

    <div class="pl-4">
        <div class="flex items-start justify-between gap-4">
            <!-- 左側：経費情報 -->
            <div class="flex-1">
                <div class="flex items-center gap-2 mb-2">
                    <span class="text-2xl">{categoryIcon()}</span>
                    <span class="font-semibold text-gray-700"
                        >{expense.category}</span
                    >
                    <span class="text-sm text-gray-500"
                        >{formatDate(expense.date)}</span
                    >
                </div>

                <div
                    class="text-2xl font-bold bg-linear-to-r from-purple-600 to-pink-600 bg-clip-text text-transparent mb-2"
                >
                    {formatAmount(expense.amount)}
                </div>

                {#if expense.description}
                    <p class="text-gray-600 text-sm mb-2">
                        {expense.description}
                    </p>
                {/if}

                <!-- 領収書サムネイル -->
                {#if hasReceipt()}
                    <div class="mt-2">
                        {#if expense.receipt_url}
                            <!-- R2に保存された領収書 -->
                            {#if isReceiptImage()}
                                <!-- 画像の場合はアイコン表示（サムネイルはR2から取得が必要なため） -->
                                <button
                                    type="button"
                                    onclick={handleViewReceipt}
                                    class="inline-flex items-center gap-2 text-sm text-blue-600 hover:text-blue-800 transition-colors"
                                >
                                    🖼️ 領収書を表示
                                </button>
                            {:else}
                                <!-- PDFの場合はリンク表示 -->
                                <button
                                    type="button"
                                    onclick={handleViewReceipt}
                                    class="inline-flex items-center gap-2 text-sm text-blue-600 hover:text-blue-800 transition-colors"
                                >
                                    📎 領収書を表示
                                </button>
                            {/if}
                        {:else if expense.receipt_path && receiptThumbnailUrl}
                            <!-- ローカルファイル（後方互換性） -->
                            {#if expense.receipt_path.match(/\.(png|jpg|jpeg)$/i)}
                                <!-- 画像の場合はサムネイル表示 -->
                                <button
                                    type="button"
                                    onclick={handleViewReceipt}
                                    class="inline-block"
                                >
                                    <img
                                        src={receiptThumbnailUrl}
                                        alt="領収書サムネイル"
                                        class="h-20 w-auto rounded border-2 border-gray-200 hover:border-purple-400 transition-colors cursor-pointer"
                                    />
                                </button>
                            {:else}
                                <!-- PDFの場合はリンク表示 -->
                                <button
                                    type="button"
                                    onclick={handleViewReceipt}
                                    class="inline-flex items-center gap-2 text-sm text-blue-600 hover:text-blue-800 transition-colors"
                                >
                                    📎 領収書を表示
                                </button>
                            {/if}
                        {/if}
                    </div>
                {/if}
            </div>

            <!-- 右側：アクションボタン -->
            <div class="flex flex-col gap-2">
                <button
                    type="button"
                    onclick={() => onEdit(expense)}
                    class="btn btn-info text-sm px-3 py-1"
                    title="編集"
                >
                    ✏️ 編集
                </button>
                <button
                    type="button"
                    onclick={confirmDelete}
                    class="btn bg-red-500 hover:bg-red-600 text-white text-sm px-3 py-1"
                    title="削除"
                >
                    🗑️ 削除
                </button>
            </div>
        </div>
    </div>
</div>

<!-- 削除確認ダイアログ -->
{#if showDeleteConfirm}
    <div
        class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
    >
        <div class="card max-w-md mx-4">
            <h3 class="text-xl font-bold mb-4">削除の確認</h3>
            <p class="text-gray-700 mb-6">
                この経費を削除してもよろしいですか？<br />
                この操作は取り消せません。
            </p>
            <div class="flex gap-3">
                <button
                    type="button"
                    onclick={handleDelete}
                    class="btn bg-red-500 hover:bg-red-600 text-white flex-1"
                >
                    削除する
                </button>
                <button
                    type="button"
                    onclick={cancelDelete}
                    class="btn bg-gray-300 text-gray-700 flex-1"
                >
                    キャンセル
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    /* グラデーションホバー効果 */
    .card:hover::before {
        content: "";
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: linear-gradient(
            135deg,
            rgba(102, 126, 234, 0.05) 0%,
            rgba(118, 75, 162, 0.05) 100%
        );
        pointer-events: none;
    }
</style>
